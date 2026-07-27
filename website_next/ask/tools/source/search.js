import { normalize } from "../text.js";

const MAX_EXCERPT_CHARACTERS = 500;
const EXCERPT_WINDOW_LINES = 21;
const SOURCE_ALIASES = new Map([
  ["address", ["addr"]],
  ["addresses", ["addr", "addrs"]],
  ["transaction", ["tx"]],
  ["transactions", ["tx", "txs"]],
]);

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

/** @param {{ path: string, text: string }[]} files */
export function createSourceSearchIndex(files) {
  /** @type {Map<string, number[]>} */
  const postings = new Map();

  files.forEach((file, fileIndex) => {
    const tokens = new Set(
      normalize(`${file.path}\n${file.text}`).split(" ").filter(Boolean),
    );
    for (const token of tokens) {
      const matches = postings.get(token);
      if (matches) matches.push(fileIndex);
      else postings.set(token, [fileIndex]);
    }
  });

  return {
    files,
    paths: files.map((file) => normalize(file.path)),
    postings,
  };
}

/** @param {string} left @param {string} right */
function relatedToken(left, right) {
  if (left === right) return true;
  /** @param {string} value */
  const stems = (value) => {
    const values = new Set([value]);
    if (value.length > 5 && value.endsWith("ies")) values.add(`${value.slice(0, -3)}y`);
    for (const suffix of ["ation", "tion", "ion", "ing", "ed", "es", "s"]) {
      if (value.length - suffix.length >= 4 && value.endsWith(suffix)) {
        values.add(value.slice(0, -suffix.length));
      }
    }
    for (const stem of [...values]) {
      if (stem.length >= 5 && stem.endsWith("e")) values.add(stem.slice(0, -1));
    }
    return values;
  };
  const leftStems = stems(left);
  const rightStems = stems(right);
  return [...leftStems].some((value) => rightStems.has(value));
}

/** @param {string} text @param {string[]} tokens @param {number[]} weights */
function bestExcerptLine(text, tokens, weights) {
  const lines = text.split("\n");
  const normalized = lines.map((line) => normalize(line));
  let bestLine = 1;
  let bestScore = 0;

  for (let start = 0; start < lines.length; start += 1) {
    const words = new Set(
      normalized
        .slice(start, start + EXCERPT_WINDOW_LINES)
        .join(" ")
        .split(" ")
        .filter(Boolean),
    );
    const score = tokens.reduce((sum, token, index) =>
      words.has(token) ? sum + weights[index] : sum, 0);
    if (!score || score < bestScore) continue;
    bestScore = score;
    bestLine = start + Math.ceil(EXCERPT_WINDOW_LINES / 2);
  }
  return Math.min(bestLine, lines.length);
}

/** @param {string} text @param {string[]} tokens */
function implementationLine(text, tokens) {
  const lines = text.split("\n");
  for (let index = 0; index < lines.length; index += 1) {
    const window = lines.slice(index, index + 5).join("\n");
    if (!/[+\-*/]/.test(window)) continue;
    if (tokens.some((token) =>
      new RegExp(`\\b(?:const|let)\\s+${token}\\s*=`).test(lines[index])
    )) return index + 1;
  }
  return undefined;
}

/**
 * @param {ReturnType<typeof createSourceSearchIndex>} index
 * @param {string} token
 */
function candidateFiles(index, token) {
  const exact = index.postings.get(token);
  const matches = new Set(exact ?? []);
  if (token.length < 5) return [...matches];

  for (const [candidate, files] of index.postings) {
    if (candidate === token) continue;
    if (!relatedToken(candidate, token)) continue;
    for (const file of files) matches.add(file);
  }
  return [...matches];
}

/**
 * @param {ReturnType<typeof createSourceSearchIndex>} index
 * @param {string} rawQuery
 * @param {string} [pathPrefix]
 */
export function searchSource(index, rawQuery, pathPrefix = "") {
  const query = normalize(rawQuery);
  if (!query) throw new Error("Search query is empty");

  const words = query.split(" ");
  const tokens = [...new Set(
    words.flatMap((word) => [word, ...(SOURCE_ALIASES.get(word) ?? [])]),
  )];
  const seeksImplementation =
    /\b(?:calculat|comput|formula|implement|source)\w*\b/.test(query) ||
    /^(?:how|where)\b/.test(query);
  const tokenMatches = tokens
    .map((token) => ({ token, files: candidateFiles(index, token) }))
    .filter(({ files }) => files.length)
    .map(({ token, files }) => ({
      token,
      files,
      weight: Math.log2((index.files.length + 1) / (files.length + 1)) + 1,
    }));
  if (!tokenMatches.length) return [];
  const maxWeight = Math.max(...tokenMatches.map(({ weight }) => weight));
  const implementationTokens = tokenMatches
    .filter(({ weight }) => weight >= maxWeight * 0.45)
    .map(({ token }) => token);

  /** @type {Map<number, { weights: number[], queryTokens: string[], matched: number }>} */
  const candidates = new Map();
  for (const { token, files, weight } of tokenMatches) {
    for (const fileIndex of files) {
      const candidate = candidates.get(fileIndex) ?? {
        weights: [],
        queryTokens: [],
        matched: 0,
      };
      candidate.weights.push(weight);
      candidate.queryTokens.push(token);
      candidate.matched += 1;
      candidates.set(fileIndex, candidate);
    }
  }

  const matches = [];
  for (const [fileIndex, candidate] of candidates) {
    const file = index.files[fileIndex];
    if (pathPrefix && !file.path.startsWith(pathPrefix)) continue;

    const normalized = normalize(file.text);
    const exact = normalized.includes(query);
    const normalizedPath = index.paths[fileIndex];
    const pathTokens = normalizedPath.split(" ");
    const pathScore = tokenMatches.reduce((score, { token, weight }) =>
      pathTokens.some((pathToken) => relatedToken(pathToken, token))
        ? score + weight
        : score, 0);
    const implementation = seeksImplementation
      ? implementationLine(file.text, implementationTokens)
      : undefined;
    const line = implementation ?? bestExcerptLine(
      file.text,
      candidate.queryTokens,
      candidate.weights,
    );
    const semanticScore = [...candidate.weights]
      .sort((left, right) => right - left)
      .slice(0, 3)
      .reduce((sum, weight) => sum + weight, 0);
    const excerpt = excerptAt(file.text, line);
    const arithmetic = seeksImplementation &&
      /\blet\s+[a-zA-Z0-9_]+\s*=[^;]{0,300}[+\-*/][^;]*;/s.test(excerpt.content);
    const supportFile = /(?:^|\/)(?:tests?|examples?)(?:\/|$)|(?:^|\/)test[_-]/.test(file.path);
    const supportPenalty =
      seeksImplementation &&
        supportFile &&
        !/\b(?:example|test)\w*\b/.test(query)
        ? 15
        : 0;
    matches.push({
      path: file.path,
      score: semanticScore +
        pathScore * 6 +
        (exact ? 10 : 0) +
        (normalizedPath.includes(query) ? 10 : 0) +
        (implementation ? 20 : 0) +
        (arithmetic ? 10 : 0) -
        supportPenalty,
      matched: candidate.matched,
      queryTokens: candidate.queryTokens,
      pathTokens,
      ...excerpt,
    });
  }

  const ranked = matches.sort((left, right) =>
      right.score - left.score ||
      right.matched - left.matched ||
      left.path.localeCompare(right.path)
    );
  const diversified = ranked.slice(0, 3);
  const salient = [...tokenMatches]
    .sort((left, right) => right.weight - left.weight)
    .slice(0, 5);
  for (const { token } of salient) {
    const candidate = ranked
      .filter((match) => match.queryTokens.includes(token))
      .sort((left, right) =>
        Number(right.pathTokens.some((pathToken) => relatedToken(pathToken, token))) -
          Number(left.pathTokens.some((pathToken) => relatedToken(pathToken, token))) ||
        right.score - left.score
      )[0];
    if (candidate) diversified.push(candidate);
  }

  return [...new Map(diversified.map((match) => [match.path, match])).values()]
    .slice(0, 8)
    .map(({ queryTokens, pathTokens, ...match }) => match);
}
