import { createSourceSearchIndex, searchSource } from "./search.js";

const CATALOG_URL = import.meta.resolve("./catalog.jsonl.gz");

/** @type {Promise<{ sha: string, files: { path: string, text: string }[] }> | undefined} */
let snapshotPromise;

/** @type {ReturnType<typeof createSourceSearchIndex> | undefined} */
let sourceSearchIndex;

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

/** @param {(loaded: number, total: number) => void} reportProgress */
async function loadSnapshot(reportProgress) {
  const response = await fetch(CATALOG_URL);
  if (!response.ok || !response.body) {
    throw new Error(`Source catalog unavailable (${response.status})`);
  }
  const decompressed = response.body.pipeThrough(new DecompressionStream("gzip"));
  const text = await new Response(decompressed).text();
  const [rawHeader, ...lines] = text.split("\n");
  const header = /** @type {{ revision: string, count: number }} */ (JSON.parse(rawHeader));
  reportProgress(0, header.count);
  const files = lines.filter(Boolean).map((line) => {
    const [path, source] = /** @type {[string, string]} */ (JSON.parse(line));
    return { path, text: source };
  });
  reportProgress(files.length, header.count);
  return { sha: header.revision, files };
}

/** @param {(loaded: number, total: number) => void} reportProgress */
function ensureSnapshot(reportProgress) {
  snapshotPromise ??= loadSnapshot(reportProgress).catch((error) => {
    snapshotPromise = undefined;
    throw error;
  });
  return snapshotPromise;
}

/** @param {(loaded: number, total: number) => void} reportProgress */
async function prewarm(reportProgress) {
  const snapshot = await ensureSnapshot(reportProgress);
  sourceSearchIndex ??= createSourceSearchIndex(snapshot.files);
  return true;
}

/**
 * @param {{ query: string, path?: string, focus?: "definition" | "implementation" | "availability" }} args
 * @param {(loaded: number, total: number) => void} reportProgress
 */
async function search(args, reportProgress) {
  const pathPrefix = String(args.path ?? "").replace(/^\/+/, "");
  const snapshot = await ensureSnapshot(reportProgress);
  sourceSearchIndex ??= createSourceSearchIndex(snapshot.files);
  return {
    revision: snapshot.sha,
    matches: searchSource(
      sourceSearchIndex,
      String(args.query ?? ""),
      pathPrefix,
      args.focus,
    ),
  };
}

self.addEventListener("message", async (event) => {
  const { id, type, data } = event.data;
  const reportProgress = (/** @type {number} */ loaded, /** @type {number} */ total) => {
    self.postMessage({ id, status: "progress", loaded, total });
  };

  try {
    let result;
    if (type === "prewarm") result = await prewarm(reportProgress);
    else if (type === "search") result = await search(data, reportProgress);
    else throw new Error(`Unknown source request: ${type}`);
    self.postMessage({ id, status: "complete", data: result });
  } catch (error) {
    self.postMessage({ id, status: "error", data: errorMessage(error) });
  }
});
