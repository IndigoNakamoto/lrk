const MAX_EXCERPT_CHARACTERS = 500;

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
  const shortest = Math.min(left.length, right.length);
  if (shortest < 5) return false;

  let prefix = 0;
  while (prefix < shortest && left[prefix] === right[prefix]) prefix += 1;
  return left.startsWith(right) ||
    right.startsWith(left) ||
    prefix >= Math.max(4, shortest - 2);
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

  const tokens = [...new Set(query.split(" "))];
  const tokenMatches = tokens
    .map((token) => ({ token, files: candidateFiles(index, token) }))
    .filter(({ files }) => files.length)
    .map(({ token, files }) => ({
      token,
      files,
      weight: Math.log2((index.files.length + 1) / (files.length + 1)) + 1,
    }));
  if (!tokenMatches.length) return [];

  /** @type {Map<number, { weights: number[], queryTokens: string[], matched: number, excerptToken: string, excerptWeight: number }>} */
  const candidates = new Map();
  for (const { token, files, weight } of tokenMatches) {
    for (const fileIndex of files) {
      const candidate = candidates.get(fileIndex) ?? {
        weights: [],
        queryTokens: [],
        matched: 0,
        excerptToken: token,
        excerptWeight: 0,
      };
      candidate.weights.push(weight);
      candidate.queryTokens.push(token);
      candidate.matched += 1;
      if (weight > candidate.excerptWeight) {
        candidate.excerptToken = token;
        candidate.excerptWeight = weight;
      }
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
    const pathMatches = tokenMatches.filter(({ token }) =>
      pathTokens.some((pathToken) => relatedToken(pathToken, token))
    ).length;
    const source = file.text.toLowerCase();
    let rawIndex = source.indexOf(candidate.excerptToken);
    if (rawIndex < 0) {
      for (const { token } of [...tokenMatches].sort((left, right) => right.weight - left.weight)) {
        rawIndex = source.indexOf(token);
        if (rawIndex >= 0) break;
      }
    }
    const line = lineAt(file.text, Math.max(0, rawIndex));
    const semanticScore = [...candidate.weights]
      .sort((left, right) => right - left)
      .slice(0, 3)
      .reduce((sum, weight) => sum + weight, 0);
    matches.push({
      path: file.path,
      score: semanticScore +
        pathMatches * 6 +
        (exact ? 10 : 0) +
        (normalizedPath.includes(query) ? 10 : 0),
      matched: candidate.matched,
      queryTokens: candidate.queryTokens,
      pathTokens,
      ...excerptAt(file.text, line),
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
    .map(({ score, matched, queryTokens, pathTokens, ...match }) => match);
}
