import { createFormulaIndex, explainFormula } from "./formula.js";
import { createSourceSearchIndex, searchSource } from "./search.js";

const CATALOG_URL = import.meta.resolve("./catalog.jsonl.gz");
const MAX_READ_LINES = 120;
const MAX_READ_CHARACTERS = 2_500;
const READ_CONTEXT_BEFORE = 4;
const READ_CONTEXT_AFTER = 40;

/** @type {Promise<{ sha: string, files: { path: string, text: string }[] }> | undefined} */
let snapshotPromise;

/** @type {ReturnType<typeof createFormulaIndex> | undefined} */
let formulaIndex;

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

/**
 * @param {{ query: string, path?: string }} args
 * @param {(loaded: number, total: number) => void} reportProgress
 */
async function search(args, reportProgress) {
  const pathPrefix = String(args.path ?? "").replace(/^\/+/, "");
  const snapshot = await ensureSnapshot(reportProgress);
  sourceSearchIndex ??= createSourceSearchIndex(snapshot.files);
  return {
    revision: snapshot.sha,
    matches: searchSource(sourceSearchIndex, String(args.query ?? ""), pathPrefix),
  };
}

/**
 * @param {{ question: string }} args
 * @param {(loaded: number, total: number) => void} reportProgress
 */
async function explain(args, reportProgress) {
  const snapshot = await ensureSnapshot(reportProgress);
  formulaIndex ??= createFormulaIndex(snapshot.files);
  const result = explainFormula(String(args.question ?? ""), formulaIndex);
  return result ? { revision: snapshot.sha, ...result } : undefined;
}

/** @param {{ path: string, startLine: number, endLine: number }} args */
async function read(args) {
  const path = String(args.path).replace(/^\/+/, "");
  const selectedStart = Math.max(1, Math.floor(Number(args.startLine)));
  const selectedEnd = Math.max(selectedStart, Math.floor(Number(args.endLine)));
  const startLine = Math.max(1, selectedStart - READ_CONTEXT_BEFORE);
  const endLine = Math.min(
    selectedEnd + READ_CONTEXT_AFTER,
    startLine + MAX_READ_LINES - 1,
  );
  const snapshot = await ensureSnapshot(() => {});
  const text = snapshot.files.find((file) => file.path === path)?.text;
  if (text === undefined) throw new Error(`Source file not found: ${path}`);
  const lines = text.split("\n");
  if (startLine > lines.length) {
    throw new Error(`${path} only has ${lines.length} lines`);
  }
  const lastLine = Math.min(endLine, lines.length);
  const content = lines.slice(startLine - 1, lastLine).join("\n");

  return {
    revision: snapshot.sha,
    path,
    startLine,
    endLine: lastLine,
    content: content.slice(0, MAX_READ_CHARACTERS),
    truncated: content.length > MAX_READ_CHARACTERS,
  };
}

self.addEventListener("message", async (event) => {
  const { id, type, data } = event.data;
  const reportProgress = (/** @type {number} */ loaded, /** @type {number} */ total) => {
    self.postMessage({ id, status: "progress", loaded, total });
  };

  try {
    const result = type === "prewarm"
      ? await explain({ question: "" }, reportProgress)
      : type === "search"
      ? await search(data, reportProgress)
      : type === "explain"
        ? await explain(data, reportProgress)
        : await read(data);
    self.postMessage({ id, status: "complete", data: result });
  } catch (error) {
    self.postMessage({ id, status: "error", data: errorMessage(error) });
  }
});
