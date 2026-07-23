import { createChartArtifact } from "./chart.js";
import { readMetric } from "./data.js";
import { directChartCommand } from "./direct/chart.js";
import { directEvidenceFocus } from "./direct/evidence.js";
import { searchLearn } from "./learn.js";
import {
  metricByName,
  metricCategories,
  metricVariants,
  metricsByPaths,
  mentionedMetrics,
  searchMetrics,
} from "./metrics/index.js";
import { ASK_STAGE_PROMPTS } from "./prompts.js";
import { AskRefs } from "./refs.js";
import { renderData, renderEvidence } from "./render.js";
import {
  clarifyTool,
  navigateTool,
  resolveTool,
  rewriteTool,
  searchTool,
} from "./schemas.js";
import { normalize } from "./text.js";

const MAX_OPTIONS = 12;
const MAX_CATEGORIES = 24;

/** @typedef {import("../storage.js").ChartArtifact} ChartArtifact */

/**
 * @typedef {Object} MetricOption
 * @property {string} ref
 * @property {string} label
 * @property {any} metric
 */

/** @param {unknown} value @param {string} name */
function requiredString(value, name) {
  if (typeof value !== "string" || !value.trim()) throw new Error(`${name} is required`);
  return value.trim();
}

/** @param {unknown} value @param {string} name */
function requiredStrings(value, name) {
  if (
    !Array.isArray(value) ||
    !value.length ||
    value.some((item) => typeof item !== "string" || !item.trim())
  ) throw new Error(`${name} are required`);
  return [...new Set(value.map((item) => item.trim()))];
}

/** @param {unknown} value */
function uniqueRefs(value) {
  if (!Array.isArray(value) || !value.length || value.some((ref) => typeof ref !== "string")) {
    throw new Error("One or more returned references are required");
  }
  return [...new Set(value)];
}

/** @param {string} value */
function label(value) {
  return value.replaceAll("_", " ");
}

/** @param {string} value */
function isExplicitComparison(value) {
  return /\b(?:vs\.?|versus|compare|compared|comparison)\b/i.test(value);
}

/** @param {string} value */
function referencesPrevious(value) {
  return /\b(?:it|its|that|this|they|their|them|those|these|same)\b/i.test(value);
}

/** @param {string} value */
function referencesSingular(value) {
  return /\b(?:it|its|that|this)\b/i.test(value);
}

/** @param {string} request */
function isDirectValueFollowup(request) {
  const text = normalize(request);
  const hasPoint = /\b(?:current|currently|latest|now|today)\b/.test(text) ||
    /\bblock\s+\d{4,}\b/.test(text) ||
    /\b\d{4}-\d{2}-\d{2}\b/.test(text);
  const needsInterpretation = /^(?:how|why)\b/.test(text) ||
    /\b(?:available|availability|chart|cohorts?|code|explain|formula|graph|history|plot|source|trend|variants?)\b/.test(text);
  return referencesPrevious(text) && hasPoint && !needsInterpretation;
}

/** @param {string} request @param {string} proposed */
function evidenceFocus(request, proposed) {
  if (/\b(?:cohorts?|variants?|availability|available)\b/i.test(request)) return "variants";
  if (/\b(?:source|code|implemented?|implementation|calculated?|calculation|formula)\b/i.test(request)) {
    return "implementation";
  }
  return proposed;
}

/** @param {string} request */
function isDirectDefinition(request) {
  const text = normalize(request);
  const asks = /\b(?:define|explain|meaning)\b/.test(text) ||
    /^(?:what is|what are)\b/.test(text);
  const needsRouting = /\b(?:available|availability|chart|cohorts?|code|current|file|graph|history|latest|now|path|plot|source|today|trend|variants?)\b/.test(text) ||
    /\bblock\s+\d+\b/.test(text) ||
    /\b\d{4}-\d{2}-\d{2}\b/.test(text);
  return asks && !needsRouting;
}

/** @param {string[]} topics @param {() => void} onProgress */
async function exactTopicMetrics(topics, onProgress) {
  if (!topics.length) return [];

  const names = new Set(topics.map(normalize));
  const metrics = await searchMetrics(topics, MAX_OPTIONS, [], onProgress);
  return metrics.filter((metric) => names.has(normalize(metric.name)));
}

/** @param {string} request */
function directReadAction(request) {
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
function completeComparisonQueries(queries) {
  if (queries.length < 3) return queries;

  const shared = queries.at(-1) ?? "";
  const qualifiers = queries.slice(0, -1);
  if (!qualifiers.every((query) => normalize(query).split(" ").length === 1)) return queries;

  return qualifiers.map((qualifier) => `${qualifier} ${shared}`);
}

/** @param {any[]} history */
function latestChart(history) {
  for (const message of [...history].reverse()) {
    const chart = message.artifacts?.findLast?.(
      (/** @type {any} */ artifact) => artifact.type === "chart",
    );
    if (chart) return chart;
  }
  return undefined;
}

/** @param {any[]} history @returns {string[] | undefined} */
function latestMetricPaths(history) {
  for (const message of [...history].reverse()) {
    if (Array.isArray(message.metricPaths)) return message.metricPaths;

    const chart = message.artifacts?.findLast?.(
      (/** @type {any} */ artifact) => artifact.type === "chart",
    );
    if (chart) return chart.chart.series.map((/** @type {any} */ item) => item.path);
  }
  return undefined;
}

/** @param {any[]} items */
function uniqueMetricOptions(items) {
  return [...new Map(items.map((/** @type {any} */ item) => [item.ref, item])).values()];
}

/** @param {any[][]} groups */
function mergeMetricGroups(groups) {
  const output = [];
  const positions = new Map();
  const ranks = Math.max(...groups.map((group) => group.length), 0);

  for (let rank = 0; rank < ranks && output.length < MAX_OPTIONS; rank += 1) {
    for (const group of groups) {
      const metric = group[rank];
      if (!metric) continue;

      const position = positions.get(metric.path);
      if (position !== undefined) {
        const current = output[position];
        const exact = normalize(metric.name) === normalize(metric.matchedQuery ?? "");
        const currentExact = normalize(current.name) === normalize(current.matchedQuery ?? "");
        if (exact && !currentExact) output[position] = metric;
        continue;
      }
      positions.set(metric.path, output.length);
      output.push(metric);
      if (output.length === MAX_OPTIONS) break;
    }
  }
  return output;
}

/** @param {any[]} items */
function balancedOptions(items) {
  const order = ["fact", "guide", "metric", "source"];
  const groups = order
    .map((kind) => items
      .filter((item) => item.kind === kind)
      .sort((left, right) => right.score - left.score))
    .filter((group) => group.length);
  const output = [];

  for (let rank = 0; output.length < MAX_OPTIONS; rank += 1) {
    let added = false;
    for (const group of groups) {
      const item = group[rank];
      if (!item) continue;
      const { score, ...option } = item;
      output.push(option);
      added = true;
      if (output.length === MAX_OPTIONS) break;
    }
    if (!added) break;
  }
  return output;
}

export class AskToolSession {
  /** @param {import("./source/index.js").AskSource} source */
  constructor(source) {
    this.source = source;
  }

  source;
  refs = new AskRefs();
  request = "";
  /** @type {string[]} */
  previousTopics = [];
  /** @type {any[]} */
  previousMetrics = [];
  /** @type {any[]} */
  contextMetrics = [];
  query = "";
  /** @type {string[]} */
  queries = [];
  outcome = "";
  focus = "definition";
  stage = "search";
  directMatch = false;
  rewritten = false;
  rewriteChanged = false;
  comparison = false;
  /** @type {string[]} */
  categoryPaths = [];
  /** @type {any} */
  observation;
  /** @type {any[]} */
  options = [];
  /** @type {MetricOption[]} */
  metricOptions = [];
  /** @type {any} */
  formula;
  sourceSearched = false;
  /** @type {ChartArtifact | undefined} */
  activeChart;

  /** @param {string} request @param {any[]} history @param {() => void} onProgress */
  async begin(request, history, onProgress) {
    this.refs = new AskRefs();
    this.request = request.trim();
    this.query = "";
    this.queries = [];
    this.outcome = "";
    this.focus = "definition";
    this.stage = "search";
    this.directMatch = false;
    this.rewritten = false;
    this.rewriteChanged = false;
    this.comparison = false;
    this.contextMetrics = [];
    this.categoryPaths = [];
    this.observation = undefined;
    this.options = [];
    this.metricOptions = [];
    this.formula = undefined;
    this.sourceSearched = false;
    this.activeChart ??= latestChart(history);

    const paths = latestMetricPaths(history);
    if (paths === undefined) return;

    const current = this.previousMetrics.map((metric) => metric.path);
    if (paths.length === current.length && paths.every((path, index) => path === current[index])) {
      return;
    }
    const metrics = paths.length ? await metricsByPaths(paths, onProgress) : [];
    this.rememberMetricValues(metrics);
  }

  /** @param {(status: string) => void} onStatus */
  async tryDirect(onStatus) {
    const evidence = directEvidenceFocus(this.request);
    const chart = evidence
      ? undefined
      : directChartCommand(this.request, Boolean(this.activeChart));
    if (chart) {
      let metrics = await mentionedMetrics(
        this.request,
        () => onStatus("Indexing metrics…"),
      );
      if (!metrics.length) {
        if (this.previousTopics.length > 1 && referencesSingular(this.request)) {
          return undefined;
        }
        metrics = this.previousMetrics.length
          ? this.previousMetrics
          : await exactTopicMetrics(
              this.previousTopics,
              () => onStatus("Indexing metrics…"),
            );
      }

      if (metrics.length) {
        const refs = metrics.map((metric) => this.refs.issue("metric", metric, metric.path));
        this.outcome = chart.kind === "edit"
          ? "edit_existing_chart"
          : "build_requested_chart";
        onStatus("Building chart…");
        const result = this.buildChart({ refs, operation: chart.operation });
        return { output: result.output, artifacts: result.artifacts };
      }
    }

    if (evidence) {
      let metrics = await mentionedMetrics(
        this.request,
        () => onStatus("Indexing metrics…"),
      );
      if (!metrics.length && this.previousMetrics.length === 1 && referencesPrevious(this.request)) {
        metrics = this.previousMetrics;
      }

      if (metrics.length) {
        if (evidence === "implementation" && metrics.length !== 1) return undefined;
        if (evidence === "variants") {
          const supported = await Promise.all(
            metrics.map(async (metric) =>
              await metricVariants(metric, this.request) ? metric : undefined
            ),
          );
          metrics = supported.filter((metric) => metric !== undefined);
          if (!metrics.length) return undefined;
        }
        if (evidence === "implementation") {
          onStatus("Searching source…");
          this.formula = await this.source.explain(
            [this.request, ...metrics.map((metric) => metric.name)].join("\n"),
            ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
          );
          if (
            !this.formula ||
            !metrics.some((metric) => normalize(metric.name) === normalize(this.formula.fact.metric))
          ) return undefined;
        }

        this.focus = evidence;
        const refs = metrics.map((metric) => this.refs.issue("metric", metric, metric.path));
        onStatus("Inspecting results…");
        const result = await this.inspect(refs);
        return { output: result.output, artifacts: [] };
      }
    }

    if (this.previousTopics.length === 1 && isDirectValueFollowup(this.request)) {
      const action = directReadAction(this.request);
      if (action) {
        let metrics = await mentionedMetrics(
          this.request,
          () => onStatus("Indexing metrics…"),
        );
        if (!metrics.length) {
          metrics = this.previousMetrics.length
            ? this.previousMetrics
            : await exactTopicMetrics(
                this.previousTopics,
                () => onStatus("Indexing metrics…"),
              );
        }

        if (metrics.length) {
          onStatus("Reading data…");
          const results = await Promise.all(metrics.map((metric) => readMetric(metric, action)));
          this.rememberMetricValues(metrics);
          return { output: renderData(results), artifacts: [] };
        }
      }
    }

    if (!isDirectDefinition(this.request)) return undefined;

    const mentioned = await mentionedMetrics(
      this.request,
      () => onStatus("Indexing metrics…"),
    );
    if (mentioned.length > 1) return undefined;
    let [metric] = mentioned;
    if (!metric && this.previousMetrics.length === 1 && referencesPrevious(this.request)) {
      [metric] = this.previousMetrics;
    }
    if (!metric) {
      [metric] = await searchMetrics(
        [this.request],
        1,
        [],
        () => onStatus("Indexing metrics…"),
      );
    }
    if (!metric) return undefined;

    onStatus("Searching source…");
    const formula = await this.source.explain(
      `${this.request}\n${metric.name}`,
      ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
    );
    if (!formula || normalize(formula.fact.metric) !== normalize(metric.name)) return undefined;

    this.rememberMetricValues([metric]);
    return { output: formula.answer, artifacts: [] };
  }

  async tool() {
    if (this.stage === "search") {
      return searchTool(
        Boolean(this.activeChart),
        this.previousTopics.length > 0,
      );
    }
    if (this.stage === "rewrite") return rewriteTool(this.queries);
    if (this.stage === "navigate") return navigateTool(this.options);
    if (this.stage === "resolve") {
      const maxItems = this.formula
        ? 1
        : this.outcome === "explain_from_verified_facts"
          ? Math.min(this.queries.length, 3)
          : this.comparison
            ? this.outcome === "read_requested_value" ? 3 : 6
            : 1;
      const options = this.outcome === "explain_from_verified_facts"
        ? this.options
        : this.metricOptions;
      return resolveTool(options, this.outcome, Math.max(1, maxItems));
    }
    return clarifyTool();
  }

  instruction() {
    if (this.stage === "search") {
      const previous = this.previousTopics.length
        ? `\nPrevious verified topic${this.previousTopics.length === 1 ? "" : "s"}: ${this.previousTopics.join(", ")}. Reuse only when the newest request depends on it.`
        : "";
      const chart = this.activeChart
        ? `\nActive chart: ${this.activeChart.chart.title}; series: ${this.activeChart.chart.series.map((item) => item.label).join(", ")}.`
        : "";
      return `${ASK_STAGE_PROMPTS.search}${previous}${chart}`;
    }
    if (this.stage === "navigate") return ASK_STAGE_PROMPTS.navigate;
    if (this.stage === "rewrite") return ASK_STAGE_PROMPTS.rewrite;
    if (this.stage === "resolve") {
      if (this.outcome === "explain_from_verified_facts") {
        return this.directMatch
          ? `${ASK_STAGE_PROMPTS.explain}\nA trusted direct match exists. Use only recommendedRefs.`
          : ASK_STAGE_PROMPTS.explain;
      }
      if (this.outcome === "read_requested_value") return ASK_STAGE_PROMPTS.read;
      return this.outcome === "edit_existing_chart"
        ? ASK_STAGE_PROMPTS.editChart
        : ASK_STAGE_PROMPTS.chart;
    }
    return ASK_STAGE_PROMPTS.clarify;
  }

  /** @param {(status: string) => void} onStatus */
  async search(onStatus) {
    const [globalMetrics, rawMetrics, guideGroups] = await Promise.all([
      searchMetrics(
        this.queries,
        MAX_OPTIONS,
        [],
        () => onStatus("Indexing metrics…"),
      ),
      searchMetrics(
        [this.request],
        MAX_OPTIONS,
        [],
        () => onStatus("Indexing metrics…"),
      ),
      Promise.all(this.queries.map((query) => searchLearn(query, MAX_OPTIONS))),
    ]);
    const foundMetrics = mergeMetricGroups([globalMetrics, rawMetrics]);
    const contextualMetrics = this.contextMetrics.map((metric) => ({
      ...metric,
      matchedQuery: label(metric.name),
      score: 2_000,
    }));
    const metrics = [...new Map(
      [...foundMetrics, ...contextualMetrics].map((metric) => [metric.path, metric]),
    ).values()];
    const metricNames = new Set(metrics.map((metric) => metric.name));
    const guides = [...new Map(
      guideGroups.flat().map((/** @type {any} */ guide) => [guide.breadcrumbs.join("/"), guide]),
    ).values()].filter((/** @type {any} */ guide) =>
      this.outcome === "explain_from_verified_facts" ||
      guide.series.some((/** @type {any} */ series) => metricNames.has(series.name))
    );
    /** @type {any[]} */
    let sources = [];

    if (
      this.outcome === "explain_from_verified_facts" &&
      this.focus !== "variants" &&
      !this.sourceSearched
    ) {
      this.sourceSearched = true;
      onStatus("Searching source…");
      const originalMetric = foundMetrics.find((metric) =>
        normalize(metric.matchedQuery ?? "") === normalize(this.request)
      );
      this.formula = await this.source.explain(
        [this.request, originalMetric?.name, ...this.queries].filter(Boolean).join("\n"),
        ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
      );
      if (!this.formula) {
        sources = (await this.source.search(
          this.queries.join(" "),
          undefined,
          ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
        )).matches;
      } else {
        const formulaMetric = await metricByName(this.formula.fact.metric);
        if (formulaMetric && !metrics.some((metric) => metric.path === formulaMetric.path)) {
          metrics.unshift({
            ...formulaMetric,
            matchedQuery: label(this.formula.fact.metric),
            score: 2_000,
          });
        }
      }
    }

    /** @type {any[]} */
    const ranked = [];
    for (const metric of metrics) {
      const ref = this.refs.issue("metric", metric, metric.path);
      ranked.push({
        ref,
        kind: "metric",
        label: label(metric.name),
        detail: metric.path,
        matchedQuery: metric.matchedQuery,
        score: metric.score,
      });
    }
    for (const guide of guides) {
      const key = guide.breadcrumbs.join("/");
      const ref = this.refs.issue("guide", guide, key);
      ranked.push({
        ref,
        kind: "guide",
        label: guide.title,
        detail: guide.description,
        score: guide.score,
      });
    }
    for (const source of sources) {
      const ref = this.refs.issue("source", source, `${source.path}:${source.startLine}`);
      ranked.push({
        ref,
        kind: "source",
        label: `${source.path}:${source.startLine}`,
        detail: source.content,
        score: source.score,
      });
    }
    if (this.formula && !metrics.length) {
      const ref = this.refs.issue("fact", this.formula, this.formula.fact.metric);
      ranked.push({
        ref,
        kind: "fact",
        label: label(this.formula.fact.metric),
        detail: this.formula.answer,
        score: 1_000,
      });
    }

    this.options = balancedOptions(ranked);
    if (!this.options.length) {
      this.stage = "clarify";
      this.observation = { noMatch: true };
      return;
    }

    const normalizedQueries = this.queries.map(normalize);
    const exactCandidates = this.options.filter((option) =>
      option.kind === "metric" &&
      normalize(option.label) === normalize(option.matchedQuery ?? "")
    );
    const exact = exactCandidates.filter((option) => {
      const optionLabel = normalize(option.label);
      return !exactCandidates.some((candidate) => {
        const candidateLabel = normalize(candidate.label);
        return candidateLabel !== optionLabel && candidateLabel.includes(optionLabel);
      });
    });
    const exactByQuery = normalizedQueries.map((query) =>
      exact.filter((option) => normalize(option.matchedQuery ?? "") === query)
    );
    this.directMatch = exactByQuery.every((matches) => matches.length === 1);
    const directOptions = exactByQuery.flat();
    const trusted = this.directMatch ||
      this.comparison && metrics.length > 0 ||
      this.rewritten && this.rewriteChanged && metrics.length > 0 ||
      Boolean(this.formula) ||
      sources.length > 0 ||
      this.outcome !== "explain_from_verified_facts" && guides.length > 0;
    if (!trusted && !this.categoryPaths.length) {
      if (!this.rewritten) {
        this.stage = "rewrite";
        this.observation = { unmatchedQueries: this.queries };
        return;
      }
      this.options = (await metricCategories()).slice(0, MAX_CATEGORIES).map((category) => ({
        ref: this.refs.issue("category", category, category.path),
        kind: "category",
        label: category.label,
        detail: `${category.count} series; ${category.examples.join(", ")}`,
      }));
      this.stage = "navigate";
      this.observation = { options: this.options };
      return;
    }

    if (
      this.directMatch &&
      this.outcome === "explain_from_verified_facts" &&
      this.focus === "variants"
    ) {
      onStatus("Inspecting results…");
      return this.inspect(directOptions.map(({ ref }) => ref));
    }
    if (this.directMatch && this.outcome === "read_requested_value") {
      const action = directReadAction(this.request);
      if (action) {
        const refs = directOptions.map(({ ref }) => ref);
        this.rememberMetrics(refs);
        onStatus("Reading data…");
        return this.read({ refs, ...action });
      }
    }
    if (this.directMatch && this.outcome === "build_requested_chart") {
      const refs = directOptions.map(({ ref }) => ref);
      this.rememberMetrics(refs);
      onStatus("Building chart…");
      return this.buildChart({ refs });
    }

    this.stage = "resolve";
    const formulaOption = this.formula
      ? this.options.find((option) =>
        option.kind === "metric" &&
        normalize(option.label) === normalize(this.formula.fact.metric)
      )
      : undefined;
    if (this.outcome === "explain_from_verified_facts") {
      if (formulaOption) return this.inspect([formulaOption.ref]);

      this.observation = {
        recommendedRefs: (directOptions.length ? directOptions : this.options)
          .slice(0, 3)
          .map(({ ref }) => ref),
        options: this.options,
      };
      return;
    }

    await this.prepareMetricOptions();
  }

  /** @param {string[]} queries @param {(status: string) => void} onStatus */
  async rewrite(queries, onStatus) {
    this.rewriteChanged = queries.length !== this.queries.length ||
      queries.some((query, index) => normalize(query) !== normalize(this.queries[index] ?? ""));
    this.queries = queries;
    this.query = queries.join(" / ");
    this.rewritten = true;
    return await this.search(onStatus) ?? { done: false };
  }

  /** @param {string[]} selected @param {(status: string) => void} onStatus */
  async navigate(selected, onStatus) {
    const categories = selected.map((ref) => this.refs.get(ref, "category"));
    this.categoryPaths = categories.map((category) => category.path);
    const prefix = categories.map((category) => category.label).join(" ");
    this.queries = this.queries.map((query) => `${prefix} ${query}`);
    this.query = this.queries.join(" / ");
    return await this.search(onStatus) ?? { done: false };
  }

  async prepareMetricOptions() {
    /** @type {MetricOption[]} */
    const metrics = [];

    for (const option of this.options) {
      const kind = this.refs.kind(option.ref);
      if (kind === "metric") {
        const metric = this.refs.get(option.ref, "metric");
        metrics.push({ ref: option.ref, label: metric.label ?? label(metric.name), metric });
      } else if (kind === "guide") {
        const guide = this.refs.get(option.ref, "guide");
        for (const series of guide.series) {
          const metric = await metricByName(series.name);
          if (!metric) continue;
          const value = { ...metric, label: series.label };
          const ref = this.refs.issue("metric", value, metric.path);
          metrics.push({ ref, label: series.label, metric: value });
        }
      }
    }

    this.metricOptions = uniqueMetricOptions(metrics).slice(0, MAX_OPTIONS);
    if (!this.metricOptions.length) {
      this.stage = "clarify";
      this.observation = { noMetric: true };
      return;
    }
    this.observation = {
      verifiedMetrics: this.metricOptions.map(({ ref, label: metricLabel, metric }) => ({
        ref,
        label: metricLabel,
        path: metric.path,
        unit: metric.suggestedUnit,
      })),
    };
  }

  /** @param {string[]} selected */
  async inspect(selected) {
    const evidence = {
      facts: /** @type {string[]} */ ([]),
      sources: /** @type {string[]} */ ([]),
      excerpts: /** @type {any[]} */ ([]),
    };
    /** @type {string[]} */
    const topics = [];
    /** @type {any[]} */
    const rememberedMetrics = [];

    for (const ref of selected) {
      const kind = this.refs.kind(ref);
      if (kind === "fact") {
        const fact = this.refs.get(ref, "fact");
        topics.push(label(fact.fact.metric));
        const metric = await metricByName(fact.fact.metric);
        if (metric) rememberedMetrics.push(metric);
        evidence.facts.push(fact.answer);
        evidence.sources.push(`${fact.fact.path}:${fact.fact.line}`);
      } else if (kind === "source") {
        const match = this.refs.get(ref, "source");
        const excerpt = await this.source.read(match.path, match.startLine, match.endLine);
        evidence.excerpts.push(excerpt);
        evidence.sources.push(`${excerpt.path}:${excerpt.startLine}-${excerpt.endLine}`);
      } else if (kind === "guide") {
        const guide = this.refs.get(ref, "guide");
        if (guide.description) evidence.facts.push(guide.description);
        topics.push(...guide.series.map((/** @type {any} */ series) => label(series.name)));
        for (const series of guide.series) {
          const metric = await metricByName(series.name);
          if (metric) rememberedMetrics.push(metric);
        }
      } else if (kind === "metric") {
        const metric = this.refs.get(ref, "metric");
        topics.push(label(metric.name));
        rememberedMetrics.push(metric);
        const variants = this.focus === "variants"
          ? await metricVariants(metric, this.query)
          : undefined;
        if (variants) {
          const groups = variants.groups
            .map((group) => `${group.family}: ${group.examples.join(", ")}`)
            .join("; ");
          evidence.facts.push(
            `${label(metric.name)} has ${variants.totalSeries} available series. Cohorts: ${groups}.`,
          );
        }
      }
    }

    if (this.formula && selected.some((ref) => this.refs.kind(ref) === "metric")) {
      evidence.facts.unshift(this.formula.answer);
      evidence.sources.push(`${this.formula.fact.path}:${this.formula.fact.line}`);
    }

    if (rememberedMetrics.length) {
      this.rememberMetricValues(rememberedMetrics);
    } else {
      this.previousMetrics = [];
      this.previousTopics = [...new Set(topics.length ? topics : this.queries)].slice(0, 4);
    }
    return { done: true, output: renderEvidence(evidence) };
  }

  /** @param {Record<string, unknown>} action */
  async read(action) {
    const selected = uniqueRefs(action.refs);
    const block = this.request.match(/\b(?:block\s*)?(\d{4,})\b/i)?.[1];
    const date = this.request.match(/\b\d{4}-\d{2}-\d{2}\b/)?.[0];
    const request = action.mode === "at" && action.at === undefined
      ? { ...action, at: block ?? date }
      : action;
    const results = await Promise.all(
      selected.map((ref) => readMetric(this.refs.get(ref, "metric"), request)),
    );
    return { done: true, output: renderData(results) };
  }

  /** @param {Record<string, unknown>} action */
  buildChart(action) {
    const selected = uniqueRefs(action.refs);
    const chosen = selected.map((ref) => this.refs.get(ref, "metric"));
    const knownMetrics = new Map(
      [...this.previousMetrics, ...chosen].map((metric) => [metric.path, metric]),
    );
    const existingChart = this.outcome === "edit_existing_chart"
      ? this.activeChart
      : undefined;
    const operation = typeof action.operation === "string" ? action.operation : "add";
    const added = chosen.map((/** @type {any} */ metric) => ({
      path: metric.path,
      label: metric.label ?? label(metric.name),
    }));
    const prior = existingChart?.chart.series ?? [];
    const paths = new Set(added.map(({ path }) => path));
    const series = !existingChart || operation === "replace"
      ? added
      : operation === "remove"
        ? prior.filter((item) => !paths.has(item.path))
        : [...prior, ...added.filter((item) => !prior.some((old) => old.path === item.path))];
    if (!series.length) throw new Error("A chart needs at least one series");

    const inferredUnit = chosen.map((metric) => metric.suggestedUnit).find(Boolean);
    const artifact = createChartArtifact({
      title: typeof action.title === "string" && action.title.trim()
        ? action.title.trim()
        : existingChart
          ? existingChart.chart.title
          : chosen.map((metric) => metric.label ?? label(metric.name)).join(" and "),
      unit: existingChart ? existingChart.chart.unit : inferredUnit,
      series,
    });
    this.activeChart = artifact;
    this.rememberMetricValues(
      artifact.chart.series.map((item) => knownMetrics.get(item.path)).filter(Boolean),
    );
    return {
      done: true,
      output: `${existingChart ? "Updated" : "Built"} **${artifact.chart.title}** with ${artifact.chart.series.map((item) => item.label).join(", ")}.`,
      artifacts: [artifact],
    };
  }

  /**
   * @param {Record<string, unknown>} action
   * @param {(status: string) => void} onStatus
   */
  async execute(action, onStatus) {
    const name = requiredString(action.action, "action");
    if (this.stage === "search") {
      if (name !== "search") throw new Error("The AI chose an invalid search action");
      const context = requiredString(action.context, "context");
      const proposed = requiredStrings(action.queries, "queries");
      const cardinality = requiredString(action.cardinality, "cardinality");
      const effectiveContext = context;
      if (effectiveContext !== "new_topic" && !this.previousTopics.length) {
        throw new Error("There is no previous verified topic to reuse");
      }
      const previousTopics = [...this.previousTopics];
      const previousMetrics = [...this.previousMetrics];
      const routedQueries = effectiveContext === "reuse_previous"
        ? previousTopics
        : effectiveContext === "extend_previous"
          ? [...new Set([...previousTopics, ...proposed])]
          : proposed;
      this.contextMetrics = effectiveContext === "new_topic" ? [] : previousMetrics;
      if (effectiveContext === "new_topic") this.rememberMetricValues([]);
      this.outcome = requiredString(action.outcome, "outcome");
      if (this.outcome === "answer_general") {
        return { done: true, general: true };
      }
      this.focus = evidenceFocus(this.request, "definition");
      this.comparison = cardinality === "multiple" || isExplicitComparison(this.request);
      this.queries = this.comparison
        ? completeComparisonQueries(routedQueries)
        : routedQueries.slice(0, 1);
      this.categoryPaths = [];
      this.query = this.queries.join(" / ");
      return await this.search(onStatus) ?? { done: false };
    }
    if (this.stage === "resolve" && this.outcome === "explain_from_verified_facts") {
      if (name !== "answer") throw new Error("The AI chose an invalid evidence action");
      onStatus("Inspecting results…");
      return this.inspect(uniqueRefs(action.refs));
    }
    if (this.stage === "navigate") {
      if (name !== "navigate") throw new Error("The AI chose an invalid navigation action");
      onStatus("Narrowing search…");
      return this.navigate(uniqueRefs(action.refs), onStatus);
    }
    if (this.stage === "rewrite") {
      if (name !== "rewrite") throw new Error("The AI chose an invalid rewrite action");
      onStatus("Refining search…");
      return this.rewrite(requiredStrings(action.queries, "queries"), onStatus);
    }
    if (this.stage === "resolve" && this.outcome === "read_requested_value") {
      if (name !== "read_data") throw new Error("The AI chose an invalid data action");
      this.rememberMetrics(uniqueRefs(action.refs));
      onStatus("Reading data…");
      return this.read(action);
    }
    if (this.stage === "resolve") {
      if (name !== "build_chart" && name !== "edit_chart") {
        throw new Error("The AI chose an invalid chart action");
      }
      onStatus("Building chart…");
      return this.buildChart(action);
    }
    if (name !== "clarify") throw new Error("The AI chose an invalid clarification action");
    return { done: true, output: requiredString(action.text, "text") };
  }

  /** @param {string[]} refs */
  rememberMetrics(refs) {
    this.rememberMetricValues(refs.map((ref) => this.refs.get(ref, "metric")));
  }

  /** @param {any[]} metrics */
  rememberMetricValues(metrics) {
    this.previousMetrics = [...new Map(
      metrics.map((metric) => [metric.path, metric]),
    ).values()].slice(0, 6);
    this.previousTopics = this.previousMetrics.map((metric) => label(metric.name));
  }

  metricPaths() {
    return this.previousMetrics.map((metric) => metric.path);
  }
}
