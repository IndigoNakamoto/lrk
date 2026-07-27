import { normalize } from "../text.js";

/** @param {string} value */
export function isExplicitComparison(value) {
  return /\b(?:vs\.?|versus|compare|compared|comparison)\b/i.test(value);
}

/** @param {string} value */
export function mayRequestMultiple(value) {
  return isExplicitComparison(value) || /\b(?:and|both|together)\b/i.test(value);
}

/** @param {string} value */
export function referencesPrevious(value) {
  return /\b(?:it|its|that|this|they|their|them|those|these|same)\b/i.test(value) ||
    /^(?:and|also|what about)\b/i.test(value.trim()) ||
    /^(?:at\s+block\s+\d+|(?:at|on)\s+\d{4}(?:[- ]\d{2}){2})\b/i.test(value.trim());
}

/** @param {string} value */
export function referencesSingular(value) {
  return /\b(?:it|its|that|this)\b/i.test(value);
}

/** @param {string} value */
export function referencesPlural(value) {
  return /\b(?:both|they|their|them|those|these|together)\b/i.test(value);
}

/** @param {string} request */
export function isDirectValueFollowup(request) {
  const text = normalize(request);
  const hasPoint = /\b(?:current|currently|latest|now|today)\b/.test(text) ||
    /\bblock\s+\d{4,}\b/.test(text) ||
    /\b\d{4}-\d{2}-\d{2}\b/.test(request);
  const needsInterpretation = /^(?:how|why)\b/.test(text) ||
    /\b(?:available|availability|chart|cohorts?|code|explain|formula|graph|history|plot|source|trend|variants?)\b/.test(text);
  return referencesPrevious(text) && hasPoint && !needsInterpretation;
}

/** @param {string} request */
export function isDirectValueRequest(request) {
  const text = normalize(request);
  const hasPoint = /\b(?:current|currently|latest|now|today)\b/.test(text) ||
    /\bblock\s+\d{4,}\b/.test(text) ||
    /\b\d{4}-\d{2}-\d{2}\b/.test(request);
  const needsDifferentTool =
    /\b(?:available|availability|chart|cohorts?|code|explain|formula|graph|history|plot|source|trend|variants?|visualize|visualise)\b/.test(text) ||
    /\b(?:over|through)\s+time\b/.test(text);
  return hasPoint && !needsDifferentTool;
}

/** @param {string} request @param {string} proposed */
export function evidenceFocus(request, proposed) {
  if (/\b(?:cohorts?|variants?|availability|available)\b/i.test(request)) return "variants";
  if (/\b(?:source|code|implemented?|implementation|calculated?|calculation|formula)\b/i.test(request)) {
    return "implementation";
  }
  return proposed;
}

/** @param {string} request */
export function isDirectDefinition(request) {
  const text = normalize(request);
  const asks = /\b(?:define|explain|meaning)\b/.test(text) ||
    /^(?:what is|what are)\b/.test(text) ||
    /^what does\b.+\bmean\b/.test(text) ||
    /\bmeans?\s+what\b/.test(text);
  const needsRouting = /\b(?:available|availability|chart|cohorts?|code|current|file|graph|history|latest|now|path|plot|source|today|trend|variants?)\b/.test(text) ||
    /\bblock\s+\d+\b/.test(text) ||
    /\b\d{4}-\d{2}-\d{2}\b/.test(request);
  return asks && !needsRouting;
}

/** @param {string} request */
export function directReadAction(request) {
  const block = request.match(/\bblock\s+(\d{4,})\b/i)?.[1];
  const dates = [...request.matchAll(/\b\d{4}-\d{2}-\d{2}\b/g)].map(([date]) => date);
  if (block) return { mode: "at", index: "height", at: block };
  if (dates.length === 1) return { mode: "at", index: "day1", at: dates[0] };
  if (dates.length > 1 || /\b(?:ago|before|after|between|from|last|previous|since|yesterday)\b/i.test(request)) {
    return undefined;
  }
  return { mode: "latest" };
}

/** @param {string[]} queries */
export function completeComparisonQueries(queries) {
  if (queries.length < 3) return queries;

  const shared = queries.at(-1) ?? "";
  const qualifiers = queries.slice(0, -1);
  if (!qualifiers.every((query) => normalize(query).split(" ").length === 1)) return queries;

  return qualifiers.map((qualifier) => `${qualifier} ${shared}`);
}
