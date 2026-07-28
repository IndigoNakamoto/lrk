import { searchApi } from "../api/index.js";
import { searchLearn } from "../learn.js";
import {
  mentionedMetricNames,
  metricByName,
  metricVariants,
  searchMetrics,
} from "../metrics/index.js";
import { normalize } from "../text.js";
import {
  explicitArguments,
  hasRequiredArguments,
} from "../api/routing.js";

const MAX_METRICS = 5;
const MAX_API = 4;
const MAX_SOURCE = 6;
const MAX_GUIDES = 2;

/** @param {string} value */
function label(value) {
  return value.replaceAll("_", " ");
}

/** @param {any[]} values @param {(value: any) => string} key */
function unique(values, key) {
  const seen = new Set();
  return values.filter((value) => {
    const itemKey = key(value);
    if (seen.has(itemKey)) return false;
    seen.add(itemKey);
    return true;
  });
}

/** @param {any} metric */
function acceptsMetric(metric) {
  return Number(metric.matchedTerms ?? 0) >= 2;
}

/** @param {any} operation @param {string} question */
function acceptsApi(operation, question) {
  const specificity = Number(operation.specificity ?? 0);
  const required = operation.parameters.some(
    (/** @type {any} */ parameter) => parameter.required,
  );
  if (!required) {
    return Number(operation.titleMatchedTerms ?? 0) > 0 &&
      specificity >= 2.5;
  }
  return specificity >= 1.5 &&
    hasRequiredArguments(operation, explicitArguments(operation, question));
}

/** @param {any} match */
function acceptsSource(match) {
  return Number(match.matched ?? 0) >=
    Math.min(2, Number(match.queryTerms ?? 0));
}

/** @param {any} guide */
function acceptsGuide(guide) {
  return Number(guide.titleCoverage ?? 0) >= 0.75;
}

/** @param {any} metric */
async function sourceMetricSubject(metric) {
  const variants = await metricVariants(metric, "");
  const names = variants?.series.map((/** @type {any} */ series) =>
    normalize(series.name).split(" ")
  ) ?? [];
  if (names.length < 2) return label(metric.name);

  const shortest = Math.min(...names.map((name) => name.length));
  const suffix = [];
  for (let offset = 1; offset <= shortest; offset += 1) {
    const word = names[0].at(-offset);
    if (!word || names.some((name) => name.at(-offset) !== word)) break;
    suffix.unshift(word);
  }
  return suffix.length >= 2 ? suffix.join(" ") : label(metric.name);
}

/**
 * Search generated metric, API, and guide catalogs before asking the model
 * what the request means. Repository excerpts are loaded only if selected.
 *
 * @param {Object} options
 * @param {string} options.question
 * @param {any} options.context
 * @param {import("../refs.js").AskRefs} options.refs
 * @param {(status: string) => void} options.onStatus
 */
export async function collectEvidence({
  question,
  context,
  refs,
  onStatus,
}) {
  const [foundMetrics, foundApi, searchedGuides, mentionedNames] =
    await Promise.all([
    searchMetrics(
      [question],
      MAX_METRICS,
      [],
      () => onStatus("Indexing metrics…"),
    ),
    searchApi(
      [question],
      MAX_API,
      () => onStatus("Indexing API…"),
    ),
    searchLearn(question, MAX_GUIDES),
    mentionedMetricNames(question),
  ]);
  const foundGuides = searchedGuides.filter(acceptsGuide);
  const mentionedMetrics = (await Promise.all(
    mentionedNames.map((name) => metricByName(name)),
  )).filter(Boolean);
  const foundByPath = new Map(
    foundMetrics.map((metric) => [metric.path, metric]),
  );
  const normalizedQuestion = normalize(question);
  const linkedMetrics = mentionedMetrics.map((/** @type {any} */ metric) => {
    const match = foundByPath.get(metric.path);
    const normalizedName = normalize(metric.name);
    const explicit = normalizedQuestion === normalizedName ||
      normalizedName.split(" ").length > 1 ||
      Number(match?.matchedTerms ?? 0) >= 2;
    return {
      ...metric,
      ...(match ?? {}),
      origin: explicit ? "mentioned" : "search",
    };
  });
  const variantMetrics = (await Promise.all(
    context.metrics.map((/** @type {any} */ metric) =>
      metricVariants(metric, question)
    ),
  )).flatMap((variants) =>
    variants?.series
      .filter((/** @type {any} */ { matchedTerms }) => matchedTerms > 0)
      .map((/** @type {any} */ metric) => {
        const name = label(metric.name);
        const selector = label(metric.selector);
        return {
          ...metric,
          label: name === selector || name.startsWith(`${selector} `)
            ? name
            : `${selector} ${name}`.trim(),
          origin: "variant",
        };
      }) ?? []
  );

  const metrics = unique(
    [
      ...linkedMetrics.filter(({ origin }) => origin === "mentioned"),
      ...variantMetrics,
      ...context.metrics.map((/** @type {any} */ metric) => ({
        ...metric,
        origin: "context",
      })),
      ...context.recentMetrics.map((/** @type {any} */ metric) => ({
        ...metric,
        origin: "recent",
      })),
      ...linkedMetrics.filter(({ origin }) => origin !== "mentioned"),
      ...foundMetrics.filter(acceptsMetric),
    ],
    (metric) => metric.path,
  ).slice(0, MAX_METRICS);
  const api = unique(
    [
      ...(context.api ? [context.api.operation] : []),
      ...foundApi.filter((operation) => acceptsApi(operation, question)),
    ],
    (operation) => operation.key,
  ).slice(0, MAX_API);
  const guides = unique(
    foundGuides,
    (guide) => guide.breadcrumbs.join("/"),
  ).slice(0, MAX_GUIDES);

  const metricOptions = metrics.map((metric) => ({
    ref: refs.issue(
      "metric",
      metric,
      metric.path,
      metric.label ?? label(metric.name),
    ),
    label: metric.label ?? label(metric.name),
    metric,
    origin: metric.origin ?? "search",
  }));
  const apiOptions = api.map((operation) => ({
    ref: refs.issue("api", operation, operation.key, operation.label),
    label: operation.label,
    operation,
  }));
  const sourceOptions = context.source.map((/** @type {any} */ match) => ({
    ref: refs.issue(
      "source",
      match,
      `${match.revision}:${match.path}:${match.startLine}`,
      match.path.split("/").at(-1) ?? match.path,
    ),
    label: `${match.path}:${match.startLine}`,
    source: match,
  }));
  const guideOptions = guides.map((guide) => ({
    ref: refs.issue(
      "guide",
      guide,
      guide.breadcrumbs.join("/"),
      guide.title,
    ),
    label: guide.title,
    guide,
  }));

  return {
    metricOptions,
    apiOptions,
    sourceOptions,
    guideOptions,
    context,
  };
}

/**
 * Source retrieval is intentionally lazy. Routing, API reads, metric values,
 * and chart edits do not need repository excerpts.
 *
 * @param {Object} options
 * @param {string} options.question
 * @param {any} options.evidence
 * @param {import("../source/index.js").AskSource} options.source
 * @param {import("../refs.js").AskRefs} options.refs
 * @param {(status: string) => void} options.onStatus
 */
export async function collectSourceOptions({
  question,
  evidence,
  source,
  refs,
  onStatus,
}) {
  const metric = evidence.metricOptions[0]?.metric;
  const metricSubject = metric
    ? await sourceMetricSubject(metric)
    : undefined;
  const queries = [
    ...(metricSubject
      ? [{
          query: metricSubject,
          focus: /** @type {const} */ ("definition"),
        }, {
          query: metricSubject,
          focus: /** @type {const} */ ("implementation"),
        }]
      : []),
    { query: question, focus: undefined },
  ].filter((value, index, values) =>
    values.findIndex((candidate) =>
      candidate.query === value.query && candidate.focus === value.focus
    ) === index
  );
  onStatus("Searching source…");
  const results = await Promise.all(
    queries.map(({ query, focus }) =>
      source.search(
        query,
        undefined,
        focus,
        ({ loaded, total }) =>
          onStatus(`Indexing source · ${loaded} / ${total}`),
      )
    ),
  );
  const sources = unique(
    [
      ...results.flatMap((result) =>
        result.matches
          .filter(acceptsSource)
          .map((/** @type {any} */ match) => ({
            ...match,
            revision: result.revision,
          }))
      ).sort((left, right) =>
        Number(right.score ?? 0) - Number(left.score ?? 0)
      ),
      ...evidence.context.source,
    ],
    (match) => `${match.revision}:${match.path}:${match.startLine}`,
  ).slice(0, MAX_SOURCE);
  return sources.map((match) => ({
    ref: refs.issue(
      "source",
      match,
      `${match.revision}:${match.path}:${match.startLine}`,
      match.path.split("/").at(-1) ?? match.path,
    ),
    label: `${match.path}:${match.startLine}`,
    source: match,
  }));
}
