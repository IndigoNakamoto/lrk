import { normalize } from "../text.js";

const VARIANTS = /\b(?:availability|available|cohorts?|variants?)\b/;
const IMPLEMENTATION = /\b(?:calculated?|calculation|code|formula|implemented?|implementation|source)\b/;

/** @param {string} request */
export function directEvidenceFocus(request) {
  const text = normalize(request);
  if (IMPLEMENTATION.test(text)) return "implementation";
  if (VARIANTS.test(text)) return "variants";
  return undefined;
}
