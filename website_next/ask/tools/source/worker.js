import { createFormulaIndex, explainFormula } from "./formula.js";

const REPOSITORY = "bitcoinresearchkit/brk";
const TREE_URL = `https://api.github.com/repos/${REPOSITORY}/git/trees/main?recursive=1`;
const RAW_URL = `https://raw.githubusercontent.com/${REPOSITORY}`;
const DATABASE_NAME = "bitview-ask-source-v1";
const STORE_NAME = "snapshots";
const MAX_FILE_SIZE = 128_000;
const MAX_READ_LINES = 120;
const MAX_READ_CHARACTERS = 2_500;
const MAX_EXCERPT_CHARACTERS = 500;
const CONCURRENCY = 12;
const SEARCH_STOPWORDS = new Set([
  "a",
  "about",
  "and",
  "bitview",
  "bitcoin",
  "brk",
  "code",
  "define",
  "does",
  "explain",
  "for",
  "how",
  "in",
  "is",
  "mean",
  "of",
  "repo",
  "repository",
  "source",
  "the",
  "what",
  "where",
  "why",
  "work",
  "works",
]);
const EXTENSIONS = new Set([
  "css",
  "html",
  "js",
  "json",
  "md",
  "mjs",
  "py",
  "rs",
  "sh",
  "toml",
  "ts",
  "yaml",
  "yml",
]);
const EXCLUDED_PREFIXES = [
  ".git/",
  ".github/",
  "modules/",
  "packages/brk_client/brk_client/",
  "target/",
  "website/assets/",
  "website_next/modules/",
];
const EXCLUDED_FILES = new Set([
  "docs/CHANGELOG.md",
  "website/scripts/options/scalar.js",
]);

/** @type {Promise<{ sha: string, entries: { path: string, size: number }[] }> | undefined} */
let treePromise;

/** @type {Promise<{ sha: string, files: { path: string, text: string }[] }> | undefined} */
let snapshotPromise;

/** @type {ReturnType<typeof createFormulaIndex> | undefined} */
let formulaIndex;

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

/** @param {string} path */
function extension(path) {
  return path.slice(path.lastIndexOf(".") + 1).toLowerCase();
}

/** @param {{ type: string, path: string, size?: number }} item */
function isSource(item) {
  return (
    item.type === "blob" &&
    Number(item.size ?? 0) <= MAX_FILE_SIZE &&
    EXTENSIONS.has(extension(item.path)) &&
    !EXCLUDED_FILES.has(item.path) &&
    !EXCLUDED_PREFIXES.some((prefix) => item.path.startsWith(prefix))
  );
}

async function loadTree() {
  treePromise ??= fetch(TREE_URL, {
    headers: { Accept: "application/vnd.github+json" },
  }).then(async (response) => {
    if (!response.ok) {
      throw new Error(`GitHub source tree unavailable (${response.status})`);
    }

    const data = await response.json();
    if (data.truncated) throw new Error("GitHub returned a truncated source tree");

    const tree = /** @type {{ type: string, path: string, size?: number }[]} */ (
      data.tree
    );
    return {
      sha: data.sha,
      entries: tree.filter(isSource).map((item) => ({
        path: item.path,
        size: Number(item.size ?? 0),
      })),
    };
  });

  return treePromise;
}

/** @param {string} sha @param {string} path */
async function fetchSource(sha, path) {
  const encodedPath = path.split("/").map(encodeURIComponent).join("/");
  const response = await fetch(`${RAW_URL}/${sha}/${encodedPath}`);
  if (!response.ok) throw new Error(`Could not fetch ${path}`);
  return response.text();
}

function openDatabase() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DATABASE_NAME, 1);
    request.addEventListener("upgradeneeded", () => {
      request.result.createObjectStore(STORE_NAME, { keyPath: "sha" });
    });
    request.addEventListener("success", () => resolve(request.result));
    request.addEventListener("error", () => reject(request.error));
  });
}

/** @param {IDBDatabase} database @param {string} sha */
function readSnapshot(database, sha) {
  return new Promise((resolve, reject) => {
    const request = database
      .transaction(STORE_NAME, "readonly")
      .objectStore(STORE_NAME)
      .get(sha);
    request.addEventListener("success", () => resolve(request.result));
    request.addEventListener("error", () => reject(request.error));
  });
}

/** @param {IDBDatabase} database @param {{ sha: string, files: { path: string, text: string }[] }} snapshot */
function writeSnapshot(database, snapshot) {
  return new Promise((resolve, reject) => {
    const transaction = database.transaction(STORE_NAME, "readwrite");
    const store = transaction.objectStore(STORE_NAME);
    store.clear();
    store.put(snapshot);
    transaction.addEventListener("complete", () => resolve(undefined));
    transaction.addEventListener("error", () => reject(transaction.error));
  });
}

/** @param {(loaded: number, total: number) => void} reportProgress */
async function loadSnapshot(reportProgress) {
  const tree = await loadTree();
  const database = /** @type {IDBDatabase} */ (await openDatabase());

  try {
    const cached = /** @type {{ sha: string, files: { path: string, text: string }[] } | undefined} */ (
      await readSnapshot(database, tree.sha)
    );
    if (cached) {
      reportProgress(cached.files.length, cached.files.length);
      return cached;
    }

    const files = new Array(tree.entries.length);
    let cursor = 0;
    let loaded = 0;
    reportProgress(loaded, tree.entries.length);

    async function download() {
      while (cursor < tree.entries.length) {
        const index = cursor;
        cursor += 1;
        const entry = tree.entries[index];
        files[index] = {
          path: entry.path,
          text: await fetchSource(tree.sha, entry.path),
        };
        loaded += 1;
        if (loaded % 20 === 0 || loaded === tree.entries.length) {
          reportProgress(loaded, tree.entries.length);
        }
      }
    }

    await Promise.all(
      Array.from({ length: Math.min(CONCURRENCY, tree.entries.length) }, download),
    );
    const snapshot = { sha: tree.sha, files };
    await writeSnapshot(database, snapshot);
    return snapshot;
  } finally {
    database.close();
  }
}

/** @param {(loaded: number, total: number) => void} reportProgress */
function ensureSnapshot(reportProgress) {
  snapshotPromise ??= loadSnapshot(reportProgress).catch((error) => {
    snapshotPromise = undefined;
    throw error;
  });
  return snapshotPromise;
}

/** @param {string} value */
function normalize(value) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** @param {string} text @param {number} index */
function lineAt(text, index) {
  return text.slice(0, index).split("\n").length;
}

/** @param {string} text @param {number} line */
function excerptAt(text, line) {
  const lines = text.split("\n");
  const start = Math.max(1, line - 3);
  const end = Math.min(lines.length, line + 3);
  return {
    startLine: start,
    endLine: end,
    content: lines
      .slice(start - 1, end)
      .join("\n")
      .slice(0, MAX_EXCERPT_CHARACTERS),
  };
}

/**
 * @param {{ query: string, path?: string }} args
 * @param {(loaded: number, total: number) => void} reportProgress
 */
async function search(args, reportProgress) {
  const rawQuery = normalize(args.query);
  if (!rawQuery) throw new Error("Search query is empty");
  const pathPrefix = String(args.path ?? "").replace(/^\/+/, "");
  const rawTokens = rawQuery.split(" ");
  const tokens = rawTokens.filter((token) => !SEARCH_STOPWORDS.has(token));
  const searchTokens = tokens.length ? tokens : rawTokens;
  const query = searchTokens.join(" ");
  const snapshot = await ensureSnapshot(reportProgress);
  const matches = [];

  for (const file of snapshot.files) {
    if (pathPrefix && !file.path.startsWith(pathPrefix)) continue;
    const normalized = normalize(file.text);
    let index = normalized.indexOf(query);
    let score = 2;

    if (index < 0 && searchTokens.every((token) => normalized.includes(token))) {
      index = normalized.indexOf(searchTokens[0]);
      score = 1;
    }
    if (index < 0) continue;

    const rawIndex = file.text.toLowerCase().indexOf(searchTokens[0]);
    const line = lineAt(file.text, Math.max(0, rawIndex));
    matches.push({
      path: file.path,
      score: score + (normalize(file.path).includes(query) ? 2 : 0),
      ...excerptAt(file.text, line),
    });
  }

  matches.sort((a, b) => b.score - a.score || a.path.localeCompare(b.path));
  return {
    revision: snapshot.sha,
    matches: matches.slice(0, 4).map(({ score, ...match }) => match),
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
  const startLine = Math.max(1, Math.floor(Number(args.startLine)));
  const requestedEnd = Math.max(startLine, Math.floor(Number(args.endLine)));
  const endLine = Math.min(requestedEnd, startLine + MAX_READ_LINES - 1);
  const tree = await loadTree();
  if (!tree.entries.some((entry) => entry.path === path)) {
    throw new Error(`Source file not found: ${path}`);
  }

  const snapshot = await snapshotPromise;
  const text = snapshot?.files.find((file) => file.path === path)?.text ??
    (await fetchSource(tree.sha, path));
  const lines = text.split("\n");
  if (startLine > lines.length) {
    throw new Error(`${path} only has ${lines.length} lines`);
  }
  const lastLine = Math.min(endLine, lines.length);
  const content = lines.slice(startLine - 1, lastLine).join("\n");

  return {
    revision: tree.sha,
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
    const result = type === "search"
      ? await search(data, reportProgress)
      : type === "explain"
        ? await explain(data, reportProgress)
        : await read(data);
    self.postMessage({ id, status: "complete", data: result });
  } catch (error) {
    self.postMessage({ id, status: "error", data: errorMessage(error) });
  }
});
