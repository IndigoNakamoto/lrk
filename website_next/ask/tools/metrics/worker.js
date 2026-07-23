import { QuickMatch, QuickMatchConfig } from "../../../modules/quickmatch-js/0.5.0/src/index.js";
import { brk } from "../../../utils/client.js";

const SEARCH_CANDIDATES = 1_024;

/** @typedef {{ path: string, name: string, endpoint: string, document: string }} CatalogMetric */

/** @param {unknown} value */
function isMetric(value) {
  if (!value || typeof value !== "object") return false;
  const by = /** @type {{ by?: Record<string, unknown> }} */ (value).by;
  return Boolean(
    by &&
      Object.values(by).some(
        (endpoint) =>
          endpoint &&
          typeof endpoint === "object" &&
          typeof /** @type {{ fetch?: unknown }} */ (endpoint).fetch === "function",
      ),
  );
}

/** @param {string} value */
function searchable(value) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_./|:-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

/** @param {string} path */
function suggestedUnit(path) {
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

function buildState() {
  /** @type {CatalogMetric[]} */
  const items = [];
  /** @type {Map<string, CatalogMetric>} */
  const byName = new Map();
  /** @type {Map<string, CatalogMetric>} */
  const byPath = new Map();
  /** @type {Map<string, CatalogMetric[]>} */
  const bySearchableName = new Map();
  /** @type {Map<string, CatalogMetric>} */
  const byDocument = new Map();
  const seen = new WeakSet();
  /** @type {{ value: unknown, keys: string[] }[]} */
  const pending = [{ value: brk.series, keys: [] }];

  while (pending.length) {
    const { value, keys } = /** @type {{ value: unknown, keys: string[] }} */ (pending.pop());
    if (!value || typeof value !== "object" || seen.has(value)) continue;
    seen.add(value);

    if (isMetric(value)) {
      const raw = /** @type {{ name?: string, by: Record<string, { path?: string }> }} */ (value);
      const path = keys.join(".");
      const name = raw.name ?? keys.at(-1) ?? "";
      const endpoint = Object.values(raw.by).find((item) => item.path)?.path ?? "";
      const document = searchable(`${name} | ${path} | ${endpoint}`);
      const metric = { path, name, endpoint, document };
      items.push(metric);
      if (!byName.has(name)) byName.set(name, metric);
      byPath.set(path, metric);
      const nameKey = searchable(name);
      const named = bySearchableName.get(nameKey) ?? [];
      named.push(metric);
      bySearchableName.set(nameKey, named);
      byDocument.set(document, metric);
    } else {
      for (const [key, child] of Object.entries(value)) {
        if (key !== "by") pending.push({ value: child, keys: [...keys, key] });
      }
    }
  }

  const config = createConfig();
  const matcher = new QuickMatch(items.map(({ document }) => document), config);
  /** @type {Map<string, { matcher: QuickMatch, config: QuickMatchConfig }>} */
  const scoped = new Map();
  return { items, byName, byPath, bySearchableName, byDocument, matcher, config, scoped };
}

/** @type {Promise<ReturnType<typeof buildState>> | undefined} */
let statePromise;

/** @param {string} id */
function state(id) {
  if (!statePromise) {
    self.postMessage({ id, status: "progress" });
    statePromise = Promise.resolve().then(buildState);
  }
  return statePromise;
}

/** @param {CatalogMetric} metric */
function publicMetric(metric) {
  return {
    path: metric.path,
    name: metric.name,
    endpoint: metric.endpoint,
    suggestedUnit: suggestedUnit(metric.path),
  };
}

/** @param {ReturnType<typeof buildState>} index @param {string[]} prefixes */
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

/** @param {ReturnType<typeof buildState>} index @param {string} query @param {number} limit @param {string[]} prefixes */
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

/** @param {ReturnType<typeof buildState>} index @param {string[]} queries @param {number} limit @param {string[]} prefixes */
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

/** @param {ReturnType<typeof buildState>} index @param {string} query */
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

/** @param {ReturnType<typeof buildState>} index */
function categories(index) {
  /** @type {Map<string, { path: string, label: string, count: number, branches: Map<string, number>[] }>} */
  const values = new Map();
  for (const metric of index.items) {
    const [root, branch] = metric.path.split(".");
    if (!root) continue;
    const category = values.get(root) ?? {
      path: root,
      label: searchable(root),
      count: 0,
      branches: [new Map(), new Map()],
    };
    category.count += 1;
    if (branch) {
      category.branches[0].set(branch, (category.branches[0].get(branch) ?? 0) + 1);
      const selector = metric.path.split(".").slice(1, 3).join(" / ");
      if (selector !== branch) {
        category.branches[1].set(selector, (category.branches[1].get(selector) ?? 0) + 1);
      }
    }
    values.set(root, category);
  }
  return [...values.values()]
    .map(({ branches, ...category }) => ({
      ...category,
      examples: branches.flatMap((level, index) => [...level]
        .sort((left, right) => right[1] - left[1])
        .slice(0, index === 0 ? 4 : 8)
        .map(([name]) => searchable(name)))
        .slice(0, 8),
    }))
    .sort((left, right) => left.label.localeCompare(right.label));
}

/** @param {ReturnType<typeof buildState>} index @param {string} name @param {string} query */
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
    series: ranked.slice(0, 16).map(({ path, name: metricName, endpoint, suggestedUnit }) => ({
      path,
      name: metricName,
      endpoint,
      suggestedUnit,
    })),
  };
}

self.addEventListener("message", async (event) => {
  const { id, type, data } = event.data;
  try {
    const index = await state(id);
    let result;
    if (type === "search") {
      result = search(index, data.queries, data.limit, data.prefixes);
    } else if (type === "mentions") {
      result = mentions(index, data.query);
    } else if (type === "categories") {
      result = categories(index);
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
