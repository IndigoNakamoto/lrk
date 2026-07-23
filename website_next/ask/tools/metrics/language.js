/** @type {[RegExp, string][]} */
const ALIASES = [
  [/\bcapitalised\b/g, "capitalized"],
  [/\ball time high\b/g, "ath"],
  [/\blong term holders?\b/g, "lth"],
  [/\bshort term holders?\b/g, "sth"],
  [/\b(?:one )?(?:bitcoin|btc) worth\b/g, "bitcoin spot price"],
];

/** @param {string} query */
function expand(query) {
  return ALIASES.reduce(
    (value, [pattern, replacement]) => value.replace(pattern, replacement),
    query.toLowerCase().replace(/[-_]+/g, " "),
  ).replace(
    /\b(\d+(?:\.\d+)?)\s+to\s+(\d+(?:\.\d+)?)\s*(btc|sats?)\b/g,
    "$1$3 to $2$3",
  );
}

/** @param {string[]} queries */
export function expandMetricQueries(queries) {
  return [...new Set(queries.flatMap((query) => {
    const expanded = expand(query);
    return expanded === query ? [query] : [expanded, query];
  }))];
}
