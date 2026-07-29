import { normalize, tokenAffinity } from "../text.js";

const MAX_EXCERPT_CHARACTERS = 900;
const EXCERPT_WINDOW_LINES = 21;
const TOKEN_AFFINITY = 0.55;
const EXCERPT_CANDIDATES = 64;
const CODE_ALIASES = new Map([
  ["function", ["fn", "func", "def"]],
  ["functions", ["fn", "func", "def"]],
]);

/** @param {string[]} words */
function searchTokens(words) {
  return [...new Set(words.flatMap((word) => [
    word,
    ...(CODE_ALIASES.get(word) ?? []),
  ]))];
}

const DECLARATION = /^\s*(?:(?:pub(?:\([^)]*\))?|export|default|async|static)\s+)*(?:fn|function|class|struct|enum|trait|interface|impl)\b/;

/** @param {string[]} lines */
function declarations(lines) {
  return lines
    .map((line, index) => DECLARATION.test(line) ? index : -1)
    .filter((index) => index >= 0);
}

/** @param {string} text @param {number} line @param {number | undefined} declaration */
function excerptAt(text, line, declaration) {
  const lines = text.split("\n");
  const declarationLine = declaration === undefined ? undefined : declaration + 1;
  const nearbyDeclaration = declarationLine !== undefined &&
    line - declarationLine < EXCERPT_WINDOW_LINES - 3;
  const start = nearbyDeclaration ? declarationLine : Math.max(1, line - 3);
  const end = Math.min(
    lines.length,
    nearbyDeclaration
      ? start + EXCERPT_WINDOW_LINES - 1
      : line + 12,
  );
  const local = lines.slice(start - 1, end).join("\n");
  const content = declarationLine !== undefined && declarationLine < start
    ? `${lines[declarationLine - 1]}\n...\n${local}`
    : local;
  return {
    startLine: declarationLine !== undefined && declarationLine < start
      ? declarationLine
      : start,
    endLine: end,
    content: content.slice(0, MAX_EXCERPT_CHARACTERS),
  };
}

/** @param {string} text @param {string} phrase */
function phraseOccurrences(text, phrase) {
  if (!phrase) return 0;
  let count = 0;
  let index = 0;
  while ((index = text.indexOf(phrase, index)) >= 0) {
    count += 1;
    index += phrase.length;
  }
  return count;
}

/** @param {string} content */
function codeOnly(content) {
  return content
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => {
      const trimmed = line.trimStart();
      return trimmed === "*" ||
          trimmed.startsWith("* ") ||
          trimmed.startsWith("*/")
        ? ""
        : line.split("//")[0];
    })
    .join("\n");
}

/** @param {string} content */
function computationWeight(content) {
  const code = codeOnly(content);
  const arithmetic = code.includes("+=") ||
    code.includes("-=") ||
    code.includes("*=") ||
    code.includes("/=") ||
    code.includes(" + ") ||
    code.includes(" - ") ||
    code.includes(" * ") ||
    code.includes(" / ");
  return arithmetic ? 2 : code.includes(".compute") ? 1 : 0;
}

/** @param {string} content @param {string} query */
function computesQueryDirectly(content, query) {
  return codeOnly(content).split("\n").some((line) => {
    return normalize(line).includes(query) && computationWeight(line) > 0;
  });
}

/** @param {string} content @param {string} query */
function containsDirectFormula(content, query) {
  return content.split("\n").some((line) => {
    const assignment = line.match(/^(.+?)(?:\+=|-=|\*=|\/=|=)(.+)$/);
    return assignment &&
      normalize(assignment[1]).includes(query) &&
      /[+\-*/×÷Σ∑]/.test(assignment[2]);
  });
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
  return left === right || tokenAffinity(left, right) >= TOKEN_AFFINITY;
}

/** @param {string} text @param {string[]} tokens @param {number[]} weights @param {string} phrase @param {boolean} preferComputation */
function bestExcerptLine(text, tokens, weights, phrase, preferComputation) {
  const lines = text.split("\n");
  const normalized = lines.map((line) => normalize(line));
  const strongest = Math.max(...weights, 1);
  const starts = declarations(lines);
  const scopes = starts.length
    ? starts.map((start, index) => ({
        start,
        end: (starts[index + 1] ?? lines.length) - 1,
      }))
    : [{ start: 0, end: lines.length - 1 }];
  const rankedScopes = scopes.map((scope) => {
    const scopeText = normalized.slice(scope.start, scope.end + 1).join(" ");
    const words = new Set(
      scopeText.split(" ")
        .filter(Boolean),
    );
    let score = tokens.reduce((sum, token, tokenIndex) =>
      [...words].some((word) => relatedToken(word, token))
        ? sum + weights[tokenIndex]
        : sum, 0);
    if (phrase && scopeText.includes(phrase)) {
      score += strongest * 3;
    }
    if (preferComputation) {
      score += computationWeight(
        lines.slice(scope.start, scope.end + 1).join("\n"),
      ) * strongest * 4;
    }
    return { ...scope, score };
  }).sort((left, right) => right.score - left.score || left.start - right.start);
  const scope = rankedScopes[0];
  let bestLine = 1;
  let bestScore = 0;

  for (let index = scope.start; index <= scope.end; index += 1) {
    const line = normalized[index];
    const nearby = normalized
      .slice(
        Math.max(0, index - Math.floor(EXCERPT_WINDOW_LINES / 2)),
        index + Math.ceil(EXCERPT_WINDOW_LINES / 2),
      )
      .join(" ");
    const lineWords = new Set(line.split(" ").filter(Boolean));
    const nearbyWords = new Set(nearby.split(" ").filter(Boolean));
    let score = tokens.reduce((sum, token, tokenIndex) => {
      if ([...lineWords].some((word) => relatedToken(word, token))) {
        return sum + weights[tokenIndex];
      }
      return [...nearbyWords].some((word) => relatedToken(word, token))
        ? sum + weights[tokenIndex] * 0.2
        : sum;
    }, 0);
    if (phrase && line.includes(phrase)) score += strongest * 3;
    if (!score || score <= bestScore) continue;
    bestScore = score;
    bestLine = index + 1;
  }
  const declaration = starts
    .filter((start) => start <= bestLine - 1)
    .at(-1);
  return { line: bestLine, declaration };
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
 * @param {"definition" | "implementation" | "availability"} [focus]
 */
export function searchSource(index, rawQuery, pathPrefix = "", focus = undefined) {
  const query = normalize(rawQuery);
  if (!query) throw new Error("Search query is empty");

  const words = query.split(" ");
  const tokens = searchTokens(words);
  const tokenMatches = tokens
    .map((token) => ({ token, files: candidateFiles(index, token) }))
    .filter(({ files }) => files.length)
    .map(({ token, files }) => ({
      token,
      files,
      weight: Math.log2((index.files.length + 1) / (files.length + 1)) + 1,
    }));
  if (!tokenMatches.length) return [];
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
    const semanticScore = [...candidate.weights]
      .sort((left, right) => right - left)
      .slice(0, 3)
      .reduce((sum, weight) => sum + weight, 0);
    matches.push({
      fileIndex,
      path: file.path,
      score: semanticScore +
        pathScore * 2 +
        (exact ? 30 : 0) +
        (normalizedPath.includes(query) ? 15 : 0),
      matched: candidate.matched,
      queryTerms: tokens.length,
      queryTokens: candidate.queryTokens,
      weights: candidate.weights,
      pathTokens,
    });
  }

  const ranked = matches.sort((left, right) =>
      right.score - left.score ||
      right.matched - left.matched ||
      left.path.localeCompare(right.path)
    )
    .slice(0, EXCERPT_CANDIDATES)
    .map((match) => {
      const file = index.files[match.fileIndex];
      const { line, declaration } = bestExcerptLine(
        file.text,
        match.queryTokens,
        match.weights,
        query,
        focus === "implementation",
      );
      const excerpt = excerptAt(file.text, line, declaration);
      const localPhraseOccurrences = phraseOccurrences(
        normalize(excerpt.content),
        query,
      );
      const declarationText = declaration === undefined
        ? ""
        : normalize(file.text.split("\n")[declaration]);
      const definitionScore = focus === "definition" &&
          (
            declarationText.includes(query) ||
            excerpt.content.split("\n").some((line) =>
              DECLARATION.test(line) && normalize(line).includes(query)
            )
          )
        ? 60
        : 0;
      const computesDirectly = focus === "implementation" &&
        computesQueryDirectly(excerpt.content, query);
      const implementationScore = computesDirectly
        ? computationWeight(excerpt.content) * 30
        : 0;
      const directImplementationScore = computesDirectly ? 40 : 0;
      const formulaScore = containsDirectFormula(excerpt.content, query)
        ? 80
        : 0;
      return {
        ...match,
        score: match.score +
          localPhraseOccurrences * 10 +
          definitionScore +
          implementationScore +
          directImplementationScore +
          formulaScore,
        phraseOccurrences: localPhraseOccurrences,
        ...excerpt,
      };
    })
    .sort((left, right) =>
      right.score - left.score ||
      right.phraseOccurrences - left.phraseOccurrences ||
      right.matched - left.matched ||
      left.path.localeCompare(right.path)
    );
  const diversified = ranked.slice(0, 8);
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
    .map(({ fileIndex, queryTokens, weights, pathTokens, ...match }) => match);
}
