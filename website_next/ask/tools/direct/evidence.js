import { normalize } from "../text.js";

const VARIANTS = /\b(?:availability|available|cohorts?|variants?)\b/;
const IMPLEMENTATION = /\b(?:calculated?|calculation|code|formula|implemented?|implementation|source)\b/;
const PRODUCT = /\b(?:bitview|brk)\b/;
const PRODUCT_QUESTION = /^(?:how|what|where|which|why)\b/;
const DATA_REQUEST =
  /\b(?:chart|current|graph|historical|history|latest|now|plot|today|trend|value|visualize|visualise)\b|\b(?:over|through)\s+time\b/;

/** @param {string} request */
export function directEvidenceFocus(request) {
  const text = normalize(request);
  if (IMPLEMENTATION.test(text)) return "implementation";
  if (VARIANTS.test(text)) return "variants";
  if (PRODUCT.test(text) && PRODUCT_QUESTION.test(text) && !DATA_REQUEST.test(text)) {
    return "implementation";
  }
  return undefined;
}
