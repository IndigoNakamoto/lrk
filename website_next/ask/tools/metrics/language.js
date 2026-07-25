/** @type {[RegExp, string][]} */
const ALIASES = [
  [/\bcapitalised\b/g, "capitalized"],
  [/\bcap price\b/g, "capitalized price"],
  [/\ball time high\b/g, "ath"],
  [/\blong term holders?\b/g, "lth"],
  [/\bshort term holders?\b/g, "sth"],
  [/\blong term\b/g, "lth"],
  [/\bshort term\b/g, "sth"],
  [/\b(lth|sth)\s+holders?\b/g, "$1"],
  [/\b(?:one )?(?:bitcoin|btc) worth\b/g, "bitcoin spot price"],
];

/** @param {string} query */
function expand(query) {
  return ALIASES.reduce(
    (value, [pattern, replacement]) => value.replace(pattern, replacement),
    query.toLowerCase().replace(/[-_]+/g, " "),
  )
    .replace(/\b(?:over|through)\s+time\b|\btime\s+series\b/g, " ")
    .replace(
      /\b(\d+(?:\.\d+)?)\s+to\s+(\d+(?:\.\d+)?)\s*(btc|sats?)\b/g,
      "$1$3 to $2$3",
    )
    .replace(/\s+/g, " ")
    .trim();
}

/** @param {string} query */
export function canonicalMetricQuery(query) {
  return expand(query);
}

/** @param {string[]} queries */
export function expandMetricQueries(queries) {
  return [...new Set(queries.flatMap((query) => {
    const expanded = expand(query);
    return expanded === query ? [query] : [expanded, query];
  }))];
}
