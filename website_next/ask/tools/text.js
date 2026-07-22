const CAMEL_BOUNDARY = /([a-z0-9])([A-Z])/g;
const NON_WORD = /[^a-z0-9%]+/g;

/** @param {unknown} value */
export function normalize(value) {
  return String(value)
    .replace(CAMEL_BOUNDARY, "$1 $2")
    .toLowerCase()
    .replace(NON_WORD, " ")
    .trim()
    .replace(/\s+/g, " ");
}

/** @param {unknown} value */
export function tokens(value) {
  return normalize(value).split(" ").filter((token) => token.length > 1);
}

/** @param {unknown} value */
function trigrams(value) {
  const padded = `  ${normalize(value)} `;
  const values = new Set();
  for (let index = 0; index < padded.length - 2; index += 1) {
    values.add(padded.slice(index, index + 3));
  }
  return values;
}

/** @param {unknown} left @param {unknown} right */
export function similarity(left, right) {
  const a = trigrams(left);
  const b = trigrams(right);
  if (!a.size || !b.size) return 0;

  let overlap = 0;
  for (const value of a) if (b.has(value)) overlap += 1;
  return (2 * overlap) / (a.size + b.size);
}

/** @param {unknown} query @param {unknown} document @param {number} [boost] */
export function relevance(query, document, boost = 0) {
  const needle = normalize(query);
  const haystack = normalize(document);
  if (!needle || !haystack) return 0;

  const words = tokens(needle);
  const exact = haystack.includes(needle) ? 30 : 0;
  const coverage = words.length
    ? words.filter((word) => haystack.includes(word)).length / words.length
    : 0;
  return boost + exact + coverage * 20 + similarity(needle, haystack) * 10;
}
