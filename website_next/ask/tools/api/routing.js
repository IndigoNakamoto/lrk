import { normalize, tokenAffinity } from "../text.js";

/** @param {string} value */
export function literalArguments(value) {
  return [...new Set(
    (value.match(/\b(?=[A-Za-z0-9:_-]*\d)[A-Za-z0-9][A-Za-z0-9:_-]*\b/g) ?? [])
      .filter((item) => item.length > 1 || /^\d$/.test(item)),
  )];
}

const API_WORD_ALIASES = new Map([
  ["funded", ["received"]],
  ["received", ["funded"]],
  ["sent", ["spent"]],
  ["spent", ["sent"]],
  ["vsize", ["virtual", "size"]],
]);

/** @param {string} request */
export function apiRequestWords(request) {
  const text = normalize(request);
  const words = new Set(text.match(/[a-z0-9]+/g) ?? []);
  for (const word of [...words]) {
    for (const alias of API_WORD_ALIASES.get(word) ?? []) words.add(alias);
  }
  if (text.includes("how many")) {
    words.add("count");
    words.add("number");
  }
  return words;
}

/** @param {string} request @param {import("./index.js").ApiOperation} operation */
export function apiResponseMatchCount(request, operation) {
  const requestWords = apiRequestWords(request);
  const nameWords = new Set(operation.response.fields.flatMap((field) =>
    normalize(field.name).match(/[a-z0-9]+/g) ?? []
  ));
  const descriptionWords = new Set(operation.response.fields.flatMap((field) =>
    (normalize(field.description).match(/[a-z0-9]+/g) ?? [])
      .filter((word) => word.length >= 4)
  ));
  return [...requestWords].filter((candidate) =>
    [...nameWords].some((word) => tokenAffinity(word, candidate) >= 0.7) ||
    candidate.length >= 4 &&
      [...descriptionWords].some((word) => tokenAffinity(word, candidate) >= 0.65)
  ).length;
}

/** @param {string} request @param {import("./index.js").ApiOperation} operation */
export function matchesApiResponse(request, operation) {
  return apiResponseMatchCount(request, operation) > 0;
}

/** @param {string} request @param {import("./index.js").ApiOperation} operation */
export function apiOperationMatchCount(request, operation) {
  const requestWords = [...apiRequestWords(request)]
    .filter((word) => word.length >= 3);
  const operationWords = new Set(
    (normalize(`${operation.summary} ${operation.path}`).match(/[a-z0-9]+/g) ?? [])
      .filter((word) => word.length >= 3),
  );
  return requestWords.filter((candidate) =>
    [...operationWords].some((word) =>
      word === candidate ||
      word === `${candidate}s` ||
      candidate === `${word}s` ||
      tokenAffinity(word, candidate) >= 0.7
    )
  ).length;
}

/** @param {string} request @param {import("./index.js").ApiOperation} operation */
export function matchesApiIntent(request, operation) {
  if (matchesApiResponse(request, operation)) return true;
  return apiOperationMatchCount(request, operation) >= 2;
}

/** @param {string} request */
function requestsExplanation(request) {
  const text = normalize(request);
  return /^(?:why|how(?! many\b| much\b))\b/.test(text) ||
    /\b(?:describe|explain|meaning|mean|works?|working)\b/.test(text) ||
    /^tell me about\b/.test(text);
}

/** @param {string} value */
export function literalType(value) {
  if (/^\d{4}-\d{2}-\d{2}$/.test(value)) return "date";
  if (/^-?\d+(?:\.\d+)?$/.test(value) && value.replace(/[^0-9]/g, "").length <= 15) {
    return "number";
  }
  if (value === "true" || value === "false") return "boolean";
  return "string";
}

/** @param {import("./index.js").ApiParameter} parameter */
function parameterType(parameter) {
  if (/^date(?:-time)?$/.test(normalize(parameter.format ?? ""))) return "date";
  const type = normalize(parameter.valueType ?? parameter.type);
  if (/\b(?:integer|number)\b/.test(type)) return "number";
  if (/\bboolean\b/.test(type)) return "boolean";
  if (/\bstring\b/.test(type)) return "string";
  return "unknown";
}

/**
 * @param {string[]} values
 * @param {import("./index.js").ApiOperation} operation
 * @param {string} [request]
 */
export function argumentAffinity(values, operation, request = "") {
  const required = operation.parameters.filter((parameter) => parameter.required);
  if (required.length !== values.length) return -1;
  if (required.some((parameter, index) => {
    const expected = parameterType(parameter);
    const actual = literalType(values[index]);
    return expected !== "unknown" && expected !== actual;
  })) return -1;
  const requestTokens = normalize(request).split(" ");
  return required.reduce((score, parameter, index) => {
    const expected = parameterType(parameter);
    const actual = literalType(values[index]);
    let next = expected === actual ? score + 2 : expected === "unknown" ? score + 1 : score;
    const valueIndex = requestTokens.indexOf(normalize(values[index]));
    const context = valueIndex > 0 ? requestTokens[valueIndex - 1] : "";
    const placeholder = `{${parameter.name}}`;
    const parts = operation.path.split("/");
    const parameterIndex = parts.indexOf(placeholder);
    const pathContext = parameterIndex > 0 ? normalize(parts[parameterIndex - 1]) : "";
    const operationWords = normalize(`${operation.summary} ${operation.path}`)
      .match(/[a-z0-9]+/g) ?? [];
    if (
      context &&
      operationWords.some((word) =>
        word === context ||
        word === `${context}s` ||
        context === `${word}s` ||
        tokenAffinity(word, context) >= 0.7
      )
    ) next += 3;
    if (
      context &&
      pathContext.split(" ").some((word) =>
        word === context ||
        word === `${context}s` ||
        context === `${word}s`
      )
    ) next += 2;
    return next;
  }, 0);
}

/**
 * @param {import("./index.js").ApiOperation[]} hints
 * @param {string[]} values
 * @param {string} request
 */
export function directApiCandidate(hints, values, request) {
  if (requestsExplanation(request)) return undefined;
  if (!values.length) {
    return hints.find((operation) =>
      operation.parameters.every((parameter) => !parameter.required) &&
      (
        apiOperationMatchCount(request, operation) >= 1 ||
        apiResponseMatchCount(request, operation) >= 2
      ) &&
      (
        (operation.matchedTerms ?? 0) >= 2 ||
        (operation.matchedTerms ?? 0) >= 1 &&
          apiResponseMatchCount(request, operation) >= 1
      ) &&
      matchesApiIntent(request, operation)
    );
  }
  const ranked = hints
    .map((operation, rank) => ({
      operation,
      rank,
      affinity: argumentAffinity(values, operation, request),
      responseMatches: apiResponseMatchCount(request, operation),
      operationMatches: apiOperationMatchCount(request, operation),
    }))
    .map((candidate) => ({
      ...candidate,
      evidence:
        candidate.affinity +
        candidate.responseMatches * 2 +
        candidate.operationMatches * 2,
    }))
    .filter(({ operation, affinity }) =>
      affinity >= 0 && (operation.matchedTerms ?? 0) > 0
    )
    .sort((left, right) =>
      right.evidence - left.evidence ||
      right.affinity - left.affinity ||
      right.responseMatches - left.responseMatches ||
      right.operationMatches - left.operationMatches ||
      left.rank - right.rank
    );
  const [first, second] = ranked;
  if (!first || !matchesApiIntent(request, first.operation)) return undefined;
  const numericAmbiguity = values.some((value) => literalType(value) === "number") &&
    second?.evidence === first.evidence &&
    second.operation.parameters
      .filter((parameter) => parameter.required)
      .map((parameter) => parameter.name)
      .join("|") !== first.operation.parameters
      .filter((parameter) => parameter.required)
      .map((parameter) => parameter.name)
      .join("|");
  return numericAmbiguity ? undefined : first.operation;
}

/**
 * @param {import("./index.js").ApiOperation} operation
 * @param {{ operation: import("./index.js").ApiOperation, arguments: Record<string, unknown> } | undefined} previous
 */
export function reusableArguments(operation, previous) {
  if (!previous) return undefined;
  const required = operation.parameters.filter((parameter) => parameter.required);
  if (!required.length && previous.operation.key !== operation.key) return undefined;
  if (!required.every((parameter) => Object.hasOwn(previous.arguments, parameter.name))) {
    return undefined;
  }
  return Object.fromEntries(
    operation.parameters
      .filter((parameter) => Object.hasOwn(previous.arguments, parameter.name))
      .map((parameter) => [parameter.name, previous.arguments[parameter.name]]),
  );
}
