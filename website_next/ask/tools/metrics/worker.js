import { QuickMatch, QuickMatchConfig } from "../../../modules/quickmatch-js/0.5.0/src/index.js";
import { metricsFromSeries } from "./series.js";

const SEARCH_CANDIDATES = 1_024;

/** @typedef {{ path: string, name: string, indexes: string[], type: string, document: string }} CatalogMetric */

/** @param {string} value */
function searchable(value) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_./|:-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

/** @param {string} path @param {string} type */
function suggestedUnit(path, type) {
  if (/(dollar|usd|cents)/i.test(type)) return "usd";
  if (/(bitcoin|btc)/i.test(type)) return "btc";
  if (/(percent|ratio)/i.test(type)) return "percent";
  if (/address/i.test(type)) return "addresses";
  if (/(utxo|output)/i.test(type)) return "utxos";
  if (/(block|height)/i.test(type)) return "blocks";
  if (/(percent|ratio|dominance|rate)/i.test(path)) return "percent";
  if (/(usd|price|cap)/i.test(path)) return "usd";
  if (/(btc|supply|value)/i.test(path)) return "btc";
  if (/(address|addr)/i.test(path)) return "addresses";
  if (/(utxo|output)/i.test(path)) return "utxos";
  if (/(block|height|epoch)/i.test(path)) return "blocks";
  return undefined;
}

/** @param {number} limit */
function createConfig(limit = SEARCH_CANDIDATES) {
  // The model supplies semantic wording, where unmatched words are usually context rather than typos.
  return new QuickMatchConfig()
    .withLimit(limit)
    .withTrigramBudget(0)
    .withMinScore(2)
    .withSeparators("_- :/.|");
}

/** @param {string} url */
async function buildState(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Series catalog unavailable (${response.status})`);
  const items = metricsFromSeries(await response.json()).map((metric) => ({
    ...metric,
    document: searchable(`${metric.name} ${metric.path}`),
  }));
  /** @type {Map<string, CatalogMetric>} */
  const byName = new Map();
  /** @type {Map<string, CatalogMetric>} */
  const byPath = new Map();
  /** @type {Map<string, CatalogMetric[]>} */
  const bySearchableName = new Map();
  /** @type {Map<string, CatalogMetric>} */
  const byDocument = new Map();
  for (const metric of items) {
    if (!byName.has(metric.name)) byName.set(metric.name, metric);
    byPath.set(metric.path, metric);
    const nameKey = searchable(metric.name);
    const named = bySearchableName.get(nameKey) ?? [];
    named.push(metric);
    bySearchableName.set(nameKey, named);
    if (!byDocument.has(metric.document)) byDocument.set(metric.document, metric);
  }

  const config = createConfig();
  const matcher = new QuickMatch(items.map(({ document }) => document), config);
  /** @type {Map<string, { matcher: QuickMatch, config: QuickMatchConfig }>} */
  const scoped = new Map();
  return { items, byName, byPath, bySearchableName, byDocument, matcher, config, scoped };
}

/** @type {Promise<Awaited<ReturnType<typeof buildState>>> | undefined} */
let statePromise;
let stateUrl = "";

/** @param {string} id @param {string} url */
function state(id, url) {
  if (!statePromise || url !== stateUrl) {
    stateUrl = url;
    self.postMessage({ id, status: "progress" });
    statePromise = buildState(url);
  }
  return statePromise;
}

/** @param {CatalogMetric} metric */
function publicMetric(metric) {
  return {
    path: metric.path,
    name: metric.name,
    indexes: metric.indexes,
    type: metric.type,
    suggestedUnit: suggestedUnit(metric.path, metric.type),
  };
}

/** @param {Awaited<ReturnType<typeof buildState>>} index @param {string[]} prefixes */
function scopedIndex(index, prefixes) {
  const inScope = (/** @type {CatalogMetric} */ metric) =>
    !prefixes.length || prefixes.some((prefix) =>
      metric.path === prefix || metric.path.startsWith(`${prefix}.`)
    );
  if (!prefixes.length) return { matcher: index.matcher, config: index.config, inScope };

  const key = [...prefixes].sort().join("|");
  let scoped = index.scoped.get(key);
  if (!scoped) {
    const config = createConfig();
    const documents = index.items.filter(inScope).map(({ document }) => document);
    scoped = { matcher: new QuickMatch(documents, config), config };
    index.scoped.set(key, scoped);
  }
  return { ...scoped, inScope };
}

/** @param {Awaited<ReturnType<typeof buildState>>} index @param {string} query @param {number} limit @param {string[]} prefixes */
function searchOne(index, query, limit, prefixes) {
  const scope = scopedIndex(index, prefixes);
  const normalizedQuery = searchable(query);
  const full = scope.matcher.matchesWith(
    normalizedQuery,
    scope.config.withLimit(SEARCH_CANDIDATES),
  );
  const fullRanks = new Map(full.map((document, rank) => [document, rank]));
  const scores = new Map();
  for (const word of [...new Set(normalizedQuery.split(" "))]) {
    if (!word) continue;
    const matches = scope.matcher.matchesWith(
      word,
      scope.config.withLimit(SEARCH_CANDIDATES),
    );
    for (const [rank, document] of matches.entries()) {
      const score = scores.get(document) ?? { matched: 0, ranks: 0 };
      score.matched += 1;
      score.ranks += rank;
      scores.set(document, score);
    }
  }
  const documents = [...new Set([...full, ...scores.keys()])]
    .sort((left, right) => {
      const a = scores.get(left) ?? { matched: 0, ranks: SEARCH_CANDIDATES };
      const b = scores.get(right) ?? { matched: 0, ranks: SEARCH_CANDIDATES };
      return b.matched - a.matched ||
        (fullRanks.get(left) ?? SEARCH_CANDIDATES) -
          (fullRanks.get(right) ?? SEARCH_CANDIDATES) ||
        a.ranks - b.ranks ||
        left.length - right.length ||
        left.localeCompare(right);
    });

  return documents
    .map((document) => index.byDocument.get(document))
    .filter((metric) => metric && scope.inScope(metric))
    .slice(0, limit)
    .map((metric, rank) => ({
      ...publicMetric(/** @type {CatalogMetric} */ (metric)),
      matchedQuery: query,
      score: 1_000 - rank,
    }));
}

/** @param {Awaited<ReturnType<typeof buildState>>} index @param {string[]} queries @param {number} limit @param {string[]} prefixes */
function search(index, queries, limit, prefixes) {
  const groups = queries.map((query) => searchOne(index, query, limit, prefixes));
  const output = [];
  const seen = new Set();

  for (let rank = 0; output.length < limit; rank += 1) {
    let added = false;
    for (const group of groups) {
      const metric = group[rank];
      if (!metric || seen.has(metric.path)) continue;
      seen.add(metric.path);
      output.push(metric);
      added = true;
      if (output.length === limit) break;
    }
    if (!added) break;
  }
  return output;
}

/** @param {Awaited<ReturnType<typeof buildState>>} index @param {string} query */
function mentions(index, query) {
  const words = searchable(query).match(/[a-z0-9]+/g) ?? [];
  /** @type {{ start: number, end: number, metric: CatalogMetric }[]} */
  const matches = [];

  for (let start = 0; start < words.length; start += 1) {
    for (let end = start + 1; end <= words.length; end += 1) {
      const named = index.bySearchableName.get(words.slice(start, end).join(" "));
      if (named?.length === 1) matches.push({ start, end, metric: named[0] });
    }
  }

  const maximal = matches.filter((match) =>
    !matches.some((candidate) =>
      candidate.start <= match.start &&
      candidate.end >= match.end &&
      candidate.end - candidate.start > match.end - match.start
    )
  );
  return [...new Map(
    maximal.map(({ metric }) => [metric.path, publicMetric(metric)]),
  ).values()];
}

/** @param {Awaited<ReturnType<typeof buildState>>} index @param {string} name @param {string} query */
function variants(index, name, query) {
  const suffix = `_${name}`;
  const candidates = index.items.filter((candidate) =>
    candidate.name === name || candidate.name.endsWith(suffix)
  );
  if (candidates.length <= 1) return undefined;

  const preferredPaths = new Map(
    index.matcher.matchesWith(searchable(query), index.config.withLimit(SEARCH_CANDIDATES))
      .map((document, rank) => [index.byDocument.get(document)?.path, rank]),
  );
  const ranked = candidates
    .map((candidate) => ({
      ...publicMetric(candidate),
      rank: preferredPaths.get(candidate.path) ?? SEARCH_CANDIDATES,
    }))
    .sort((left, right) =>
      Number(right.name === name) - Number(left.name === name) ||
      left.rank - right.rank ||
      left.path.localeCompare(right.path)
    );

  let commonSuffix = candidates[0].path.split(".");
  for (const candidate of candidates.slice(1)) {
    const path = candidate.path.split(".");
    let count = 0;
    while (
      count < commonSuffix.length &&
      count < path.length &&
      commonSuffix[commonSuffix.length - 1 - count] === path[path.length - 1 - count]
    ) count += 1;
    commonSuffix = count ? commonSuffix.slice(-count) : [];
  }

  const groups = new Map();
  for (const candidate of ranked) {
    const selector = candidate.path.split(".").slice(0, -commonSuffix.length);
    const cohortIndex = selector.indexOf("cohorts");
    const cohort = selector.slice(cohortIndex < 0 ? 0 : cohortIndex + 1);
    const family = cohort.length > 2 ? cohort.slice(0, 2).join(" / ") : cohort[0] ?? "root";
    const value = cohort.length > 2 ? cohort.slice(2).join(" / ") : cohort[1] ?? "all";
    const group = groups.get(family) ?? { family, count: 0, examples: [] };
    group.count += 1;
    if (group.examples.length < 5) group.examples.push(value);
    groups.set(family, group);
  }

  return {
    totalSeries: ranked.length,
    groups: [...groups.values()].slice(0, 8),
    series: ranked.slice(0, 16).map(({ path, name: metricName, suggestedUnit }) => ({
      path,
      name: metricName,
      suggestedUnit,
    })),
  };
}

self.addEventListener("message", async (event) => {
  const { id, type, data } = event.data;
  try {
    const index = await state(id, data.url);
    let result;
    if (type === "prewarm") {
      result = true;
    } else if (type === "search") {
      result = search(index, data.queries, data.limit, data.prefixes);
    } else if (type === "mentions") {
      result = mentions(index, data.query);
    } else if (type === "byName") {
      const metric = index.byName.get(data.name);
      result = metric ? publicMetric(metric) : undefined;
    } else if (type === "byPaths") {
      result = data.paths
        .map((/** @type {string} */ path) => index.byPath.get(path))
        .filter(Boolean)
        .map(publicMetric);
    } else if (type === "variants") {
      result = variants(index, data.name, data.query);
    } else {
      throw new Error(`Unknown metric request: ${type}`);
    }
    self.postMessage({ id, status: "complete", data: result });
  } catch (error) {
    self.postMessage({
      id,
      status: "error",
      data: error instanceof Error ? error.message : String(error),
    });
  }
});
