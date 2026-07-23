import { brk } from "../../../utils/client.js";
import { expandMetricQueries } from "./language.js";

const WORKER_URL = import.meta.resolve("./worker.js");
const FORBIDDEN_KEYS = new Set(["__proto__", "constructor", "prototype"]);

/** @typedef {{ path: string, name: string, endpoint: string, suggestedUnit?: string, matchedQuery?: string, score?: number }} CatalogMetric */

/** @param {unknown} value */
function isMetric(value) {
  if (!value || typeof value !== "object") return false;
  const by = /** @type {{ by?: Record<string, unknown> }} */ (value).by;
  return Boolean(
    by &&
      Object.values(by).some(
        (endpoint) =>
          endpoint &&
          typeof endpoint === "object" &&
          typeof /** @type {{ fetch?: unknown }} */ (endpoint).fetch === "function",
      ),
  );
}

/** @param {string} path */
function normalizePath(path) {
  const normalized = path
    .trim()
    .replace(/^brk\.series\./, "")
    .replace(/^series\./, "");
  const keys = normalized.split(".").filter(Boolean);

  if (!keys.length || keys.some((key) => FORBIDDEN_KEYS.has(key))) {
    throw new Error(`Invalid metric path: ${path}`);
  }
  return keys;
}

/** @param {typeof brk} client @param {string} path */
export function resolveMetric(client, path) {
  const keys = normalizePath(path);
  /** @type {unknown} */
  let value = client.series;

  for (const key of keys) {
    if (!value || typeof value !== "object" || !Object.hasOwn(value, key)) {
      throw new Error(`Metric not found: ${path}`);
    }
    value = /** @type {Record<string, unknown>} */ (value)[key];
  }

  if (!isMetric(value)) throw new Error(`Metric not found: ${path}`);
  return /** @type {TimeframeMetric} */ (value);
}

/** @param {string} path */
export function createMetric(path) {
  resolveMetric(brk, path);
  return (/** @type {typeof brk} */ client) => resolveMetric(client, path);
}

class MetricIndex {
  /** @type {Worker | undefined} */
  #worker;

  /** @type {Map<string, { resolve: (value: any) => void, reject: (error: Error) => void, onProgress?: () => void }>} */
  #pending = new Map();

  /** @param {"search" | "mentions" | "categories" | "byName" | "byPaths" | "variants"} type @param {Record<string, unknown>} data @param {(() => void) | undefined} [onProgress] */
  request(type, data, onProgress) {
    this.#ensureWorker();
    const id = crypto.randomUUID();

    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject, onProgress });
      this.#worker?.postMessage({ id, type, data });
    });
  }

  #ensureWorker() {
    if (this.#worker) return;
    this.#worker = new Worker(WORKER_URL, { type: "module" });
    this.#worker.addEventListener("message", this.#handleMessage);
    this.#worker.addEventListener("error", this.#handleError);
  }

  terminate() {
    const error = new Error("Metric search stopped");
    for (const request of this.#pending.values()) request.reject(error);
    this.#pending.clear();
    this.#worker?.terminate();
    this.#worker = undefined;
  }

  /** @param {MessageEvent} event */
  #handleMessage = (event) => {
    const message = event.data;
    const request = this.#pending.get(message.id);
    if (!request) return;

    if (message.status === "progress") {
      request.onProgress?.();
      return;
    }

    this.#pending.delete(message.id);
    if (message.status === "complete") request.resolve(message.data);
    else request.reject(new Error(message.data));
  };

  /** @param {ErrorEvent} event */
  #handleError = (event) => {
    const error = new Error(event.message || "The metric index failed");
    for (const request of this.#pending.values()) request.reject(error);
    this.#pending.clear();
    this.#worker?.terminate();
    this.#worker = undefined;
  };
}

const index = new MetricIndex();

/** @param {string[]} queries @param {number} [limit] @param {string[]} [prefixes] @param {(() => void) | undefined} [onProgress] @returns {Promise<CatalogMetric[]>} */
export function searchMetrics(queries, limit = 16, prefixes = [], onProgress) {
  return index.request(
    "search",
    { queries: expandMetricQueries(queries), limit, prefixes },
    onProgress,
  );
}

/** @param {string} query @param {(() => void) | undefined} [onProgress] @returns {Promise<CatalogMetric[]>} */
export function mentionedMetrics(query, onProgress) {
  return index.request("mentions", { query }, onProgress);
}

/** @returns {Promise<{ path: string, label: string, count: number, examples: string[] }[]>} */
export function metricCategories() {
  return index.request("categories", {});
}

/** @param {string} name @returns {Promise<CatalogMetric | undefined>} */
export function metricByName(name) {
  return index.request("byName", { name });
}

/** @param {string[]} paths @param {(() => void) | undefined} [onProgress] @returns {Promise<CatalogMetric[]>} */
export function metricsByPaths(paths, onProgress) {
  return index.request("byPaths", { paths }, onProgress);
}

/** @param {{ name: string }} metric @param {string} query @returns {Promise<{ totalSeries: number, groups: { family: string, examples: string[] }[], series: CatalogMetric[] } | undefined>} */
export function metricVariants(metric, query = "") {
  return index.request("variants", { name: metric.name, query });
}

export function terminateMetricIndex() {
  index.terminate();
}
