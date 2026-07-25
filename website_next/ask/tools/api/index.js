import { BRK_BASE_URL } from "../../../utils/client.js";

const WORKER_URL = import.meta.resolve("./worker.js");
const OPENAPI_URL = `${BRK_BASE_URL}/openapi.json`;

/**
 * @typedef {Object} ApiParameter
 * @property {string} name
 * @property {"path" | "query"} in
 * @property {boolean} required
 * @property {string} type
 * @property {string} [valueType]
 * @property {unknown[]} [enum]
 * @property {string} description
 *
 * @typedef {Object} ApiOperation
 * @property {string} key
 * @property {"GET"} method
 * @property {string} path
 * @property {string} label
 * @property {string} summary
 * @property {string} description
 * @property {ApiParameter[]} parameters
 * @property {{ contentType: string, type: string, description: string, fields: { name: string, type: string, required: boolean, description: string }[] }} response
 * @property {string} [matchedQuery]
 * @property {number} [matchedTerms]
 * @property {number} [score]
 */

class ApiIndex {
  /** @type {Worker | undefined} */
  #worker;

  /** @type {Map<string, { resolve: (value: any) => void, reject: (error: Error) => void, onProgress?: () => void }>} */
  #pending = new Map();

  /** @param {"prewarm" | "search" | "byKey"} type @param {Record<string, unknown>} data @param {(() => void) | undefined} [onProgress] */
  request(type, data, onProgress) {
    this.#ensureWorker();
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject, onProgress });
      this.#worker?.postMessage({ id, type, data: { ...data, url: OPENAPI_URL } });
    });
  }

  #ensureWorker() {
    if (this.#worker) return;
    this.#worker = new Worker(WORKER_URL, { type: "module" });
    this.#worker.addEventListener("message", this.#handleMessage);
    this.#worker.addEventListener("error", this.#handleError);
  }

  terminate() {
    const error = new Error("API search stopped");
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
    const error = new Error(event.message || "The API index failed");
    for (const request of this.#pending.values()) request.reject(error);
    this.#pending.clear();
    this.#worker?.terminate();
    this.#worker = undefined;
  };
}

const index = new ApiIndex();

export function prewarmApiIndex() {
  return index.request("prewarm", {});
}

/** @param {string[]} queries @param {number} [limit] @param {(() => void) | undefined} [onProgress] @returns {Promise<ApiOperation[]>} */
export function searchApi(queries, limit = 8, onProgress) {
  return index.request("search", { queries, limit }, onProgress);
}

/** @param {string} key @returns {Promise<ApiOperation | undefined>} */
export function apiByKey(key) {
  return index.request("byKey", { key });
}

export function terminateApiIndex() {
  index.terminate();
}
