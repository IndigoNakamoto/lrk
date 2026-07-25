import { createChartArtifact } from "./chart.js";
import { resolveChartUnit } from "./chart/units.js";
import { apiByKey, searchApi } from "./api/index.js";
import { executeApi } from "./api/execute.js";
import { readMetric } from "./data.js";
import { directChartCommand } from "./direct/chart.js";
import { directEvidenceFocus } from "./direct/evidence.js";
import { searchLearn } from "./learn.js";
import {
  metricByName,
  metricVariants,
  metricsByPaths,
  mentionedMetrics,
  searchMetrics,
} from "./metrics/index.js";
import { canonicalMetricQuery } from "./metrics/language.js";
import { ASK_STAGE_PROMPTS } from "./prompts.js";
import { AskRefs } from "./refs.js";
import { renderData, renderEvidence } from "./render.js";
import {
  apiResolveTool,
  clarifyTool,
  resolveTool,
  rewriteTool,
  searchTool,
} from "./schemas.js";
import { normalize } from "./text.js";

const MAX_OPTIONS = 12;
const MAX_API_OPTIONS = 6;
const MAX_API_HINTS = 12;

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
function mayRequestMultiple(value) {
  return isExplicitComparison(value) || /\b(?:and|both|together)\b/i.test(value);
}

/** @param {string} value */
function referencesPrevious(value) {
  return /\b(?:it|its|that|this|they|their|them|those|these|same)\b/i.test(value) ||
    /^(?:and|also|what about)\b/i.test(value.trim());
}

/** @param {string} value */
function referencesSingular(value) {
  return /\b(?:it|its|that|this)\b/i.test(value);
}

/** @param {string} value */
function referencesPlural(value) {
  return /\b(?:they|their|them|those|these)\b/i.test(value);
}

/** @param {string} value */
function literalArguments(value) {
  return [...new Set(
    (value.match(/\b(?=[A-Za-z0-9:_-]*\d)[A-Za-z0-9][A-Za-z0-9:_-]*\b/g) ?? [])
      .filter((item) => item.length > 1 || /^\d$/.test(item)),
  )];
}

/** @param {string} request @param {import("./api/index.js").ApiOperation} operation */
function matchesApiResponse(request, operation) {
  const words = new Set(normalize(request).match(/[a-z0-9]+/g) ?? []);
  return operation.response.fields.some((field) => {
    const names = normalize(field.name).match(/[a-z0-9]+/g) ?? [];
    if (names.some((word) => words.has(word))) return true;
    const description = normalize(field.description).match(/[a-z0-9]+/g) ?? [];
    return description.some((word) => word.length >= 4 && words.has(word));
  });
}

/** @param {string} request @param {import("./api/index.js").ApiOperation} operation */
function matchesApiIntent(request, operation) {
  if (matchesApiResponse(request, operation)) return true;
  const words = new Set(
    (normalize(request).match(/[a-z0-9]+/g) ?? []).filter((word) => word.length >= 3),
  );
  const document = new Set(
    (normalize(`${operation.summary} ${operation.path}`).match(/[a-z0-9]+/g) ?? [])
      .filter((word) => word.length >= 3),
  );
  return [...words].filter((word) => document.has(word)).length >= 2;
}

/** @param {string} value */
function literalType(value) {
  if (/^-?\d+(?:\.\d+)?$/.test(value) && value.replace(/[^0-9]/g, "").length <= 15) {
    return "number";
  }
  if (value === "true" || value === "false") return "boolean";
  return "string";
}

/** @param {import("./api/index.js").ApiParameter} parameter */
function parameterType(parameter) {
  const type = normalize(parameter.valueType ?? parameter.type);
  if (/\b(?:integer|number)\b/.test(type)) return "number";
  if (/\bboolean\b/.test(type)) return "boolean";
  if (/\bstring\b/.test(type)) return "string";
  return "unknown";
}

/**
 * Prefer candidates whose source-derived parameter schemas fit the supplied
 * literals, while retaining catalog rank for equal fits.
 * @param {string[]} values
 * @param {import("./api/index.js").ApiOperation} operation
 * @param {string} [request]
 */
function argumentAffinity(values, operation, request = "") {
  const required = operation.parameters.filter((parameter) => parameter.required);
  if (required.length !== values.length) return -1;
  const requestTokens = normalize(request).split(" ");
  return required.reduce((score, parameter, index) => {
    const expected = parameterType(parameter);
    const actual = literalType(values[index]);
    let next = expected === actual ? score + 2 : expected === "unknown" ? score + 1 : score;
    const valueIndex = requestTokens.indexOf(normalize(values[index]));
    const context = valueIndex > 0 ? requestTokens[valueIndex - 1] : "";
    const placeholder = `{${parameter.name}}`;
    const parts = operation.path.split("/");
    const parameterIndex = parts.indexOf(placeholder);
    const pathContext = parameterIndex > 0 ? normalize(parts[parameterIndex - 1]) : "";
    if (context && pathContext.split(" ").includes(context)) next += 2;
    return next;
  }, 0);
}

/**
 * @param {import("./api/index.js").ApiOperation[]} hints
 * @param {string[]} values
 * @param {string} request
 */
function directApiCandidate(hints, values, request) {
  if (!values.length) {
    return hints.find((operation) =>
      operation.parameters.every((parameter) => !parameter.required) &&
      (operation.matchedTerms ?? 0) >= 2 &&
      matchesApiIntent(request, operation)
    );
  }
  const ranked = hints
    .map((operation, rank) => ({
      operation,
      rank,
      affinity: argumentAffinity(values, operation, request),
    }))
    .filter(({ operation, affinity }) =>
      affinity >= 0 && (operation.matchedTerms ?? 0) > 0
    )
    .sort((left, right) =>
      right.affinity - left.affinity || left.rank - right.rank
    );
  const [first, second] = ranked;
  if (!first || !matchesApiIntent(request, first.operation)) return undefined;
  const numericAmbiguity = values.some((value) => literalType(value) === "number") &&
    second?.affinity === first.affinity &&
    second.operation.parameters
      .filter((parameter) => parameter.required)
      .map((parameter) => parameter.name)
      .join("|") !== first.operation.parameters
      .filter((parameter) => parameter.required)
      .map((parameter) => parameter.name)
      .join("|");
  return numericAmbiguity ? undefined : first.operation;
}

/**
 * @param {import("./api/index.js").ApiOperation} operation
 * @param {{ operation: import("./api/index.js").ApiOperation, arguments: Record<string, unknown> } | undefined} previous
 */
function reusableArguments(operation, previous) {
  if (!previous) return undefined;
  const required = operation.parameters.filter((parameter) => parameter.required);
  if (!required.length && previous.operation.key !== operation.key) return undefined;
  if (!required.every((parameter) => Object.hasOwn(previous.arguments, parameter.name))) {
    return undefined;
  }
  return Object.fromEntries(
    operation.parameters
      .filter((parameter) => Object.hasOwn(previous.arguments, parameter.name))
      .map((parameter) => [parameter.name, previous.arguments[parameter.name]]),
  );
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

/** @param {string} request */
function isDirectValueRequest(request) {
  const text = normalize(request);
  const hasPoint = /\b(?:current|currently|latest|now|today)\b/.test(text) ||
    /\bblock\s+\d{4,}\b/.test(text) ||
    /\b\d{4}-\d{2}-\d{2}\b/.test(text);
  const needsDifferentTool =
    /\b(?:available|availability|chart|cohorts?|code|explain|formula|graph|history|plot|source|trend|variants?|visualize|visualise)\b/.test(text) ||
    /\b(?:over|through)\s+time\b/.test(text);
  return hasPoint && !needsDifferentTool;
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

/**
 * Resolve coordinated metric wording only when expanding its shared suffix
 * produces two exact generated catalog names.
 * @param {string} request
 * @param {() => void} onProgress
 */
async function coordinatedMetrics(request, onProgress) {
  const expression = canonicalMetricQuery(request)
    .replace(
      /^(?:(?:compare|chart|graph|plot|show|visualize|visualise)(?: me)?|comparison of)\s+/,
      "",
    )
    .replace(/^both\s+/, "")
    .trim();
  const parts = expression
    .split(/\s+(?:and|against|versus|vs)\s+/)
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length !== 2) return [];

  const [left, right] = parts;
  const candidates = [[left, right]];
  const leftWords = left.split(" ");
  const rightWords = right.split(" ");
  for (let index = 1; index < rightWords.length; index += 1) {
    candidates.push([`${left} ${rightWords.slice(index).join(" ")}`, right]);
  }
  for (let index = 1; index < leftWords.length; index += 1) {
    candidates.push([left, `${right} ${leftWords.slice(index).join(" ")}`]);
  }

  for (const topics of candidates) {
    const metrics = await exactTopicMetrics(topics, onProgress);
    if (new Set(metrics.map((metric) => metric.path)).size === 2) return metrics;
  }
  return [];
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

/** @param {any[]} history */
function recentMetricPaths(history) {
  /** @type {string[]} */
  const paths = [];
  for (const message of [...history].reverse()) {
    const remembered = Array.isArray(message.metricPaths)
      ? message.metricPaths
      : message.artifacts?.findLast?.(
          (/** @type {any} */ artifact) => artifact.type === "chart",
        )?.chart.series.map((/** @type {any} */ item) => item.path);
    for (const path of remembered ?? []) {
      if (!paths.includes(path)) paths.push(path);
      if (paths.length === 6) return paths;
    }
  }
  return paths;
}

/** @param {any[]} history */
function latestApiContext(history) {
  for (const message of [...history].reverse()) {
    if (message.apiContext) return message.apiContext;
  }
  return undefined;
}

/** @param {any} formula */
function formulaCitation(formula) {
  return {
    revision: formula.revision,
    path: formula.fact.path,
    startLine: formula.fact.line,
  };
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
  recentMetrics = [];
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
  /** @type {any} */
  observation;
  /** @type {any[]} */
  options = [];
  /** @type {MetricOption[]} */
  metricOptions = [];
  /** @type {{ ref: string, label: string, operation: import("./api/index.js").ApiOperation }[]} */
  apiOptions = [];
  /** @type {import("./api/index.js").ApiOperation[]} */
  apiHints = [];
  /** @type {import("./api/index.js").ApiOperation[]} */
  apiCandidates = [];
  /** @type {import("./api/index.js").ApiOperation | undefined} */
  directApiHint;
  /** @type {{ operation: import("./api/index.js").ApiOperation, arguments: Record<string, unknown> } | undefined} */
  previousApi;
  reuseApiContext = false;
  /** @type {any} */
  formula;
  sourceSearched = false;
  requiresTools = false;
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
    this.observation = undefined;
    this.options = [];
    this.metricOptions = [];
    this.apiOptions = [];
    this.apiHints = [];
    this.apiCandidates = [];
    this.directApiHint = undefined;
    this.reuseApiContext = false;
    this.formula = undefined;
    this.sourceSearched = false;
    this.requiresTools = false;
    this.activeChart ??= latestChart(history);

    const apiContext = latestApiContext(history);
    if (apiContext?.key) {
      const operation = await apiByKey(apiContext.key);
      this.previousApi = operation
        ? { operation, arguments: apiContext.arguments ?? {} }
        : undefined;
    } else {
      this.previousApi = undefined;
    }

    const dependsOnPrevious = Boolean(this.previousApi) && referencesPrevious(this.request);
    const apiQuery = dependsOnPrevious
      ? `${this.request} ${this.previousApi?.operation.summary || this.previousApi?.operation.label}`
      : this.request;
    const literals = literalArguments(this.request);
    this.apiHints = await searchApi([apiQuery], MAX_API_HINTS, onProgress).catch(() => []);
    this.apiCandidates = literals.length
      ? this.apiHints.filter((operation) =>
          argumentAffinity(literals, operation, this.request) >= 0 &&
          (operation.matchedTerms ?? 0) > 0
        )
      : [];
    this.directApiHint = directApiCandidate(this.apiHints, literals, this.request);
    this.requiresTools = Boolean(this.directApiHint) || this.apiCandidates.length > 0;

    if (this.previousApi && referencesPrevious(this.request)) {
      this.requiresTools = true;
      const contextual = this.apiHints.find((operation) =>
        (operation.matchedTerms ?? 0) >= 2 &&
        matchesApiIntent(this.request, operation) &&
        reusableArguments(operation, this.previousApi) !== undefined
      );
      if (contextual) {
        this.directApiHint = contextual;
      } else if (matchesApiResponse(this.request, this.previousApi.operation)) {
        this.directApiHint = this.previousApi.operation;
      }
    }

    const focusPaths = latestMetricPaths(history);
    if (focusPaths === undefined) return;
    const recentPaths = recentMetricPaths(history);
    const currentFocus = this.previousMetrics.map((metric) => metric.path);
    const currentRecent = this.recentMetrics.map((metric) => metric.path);
    if (
      focusPaths.length === currentFocus.length &&
      focusPaths.every((path, index) => path === currentFocus[index]) &&
      recentPaths.length === currentRecent.length &&
      recentPaths.every((path, index) => path === currentRecent[index])
    ) {
      return;
    }
    const paths = [...new Set([...focusPaths, ...recentPaths])];
    const metrics = paths.length ? await metricsByPaths(paths, onProgress) : [];
    const byPath = new Map(metrics.map((metric) => [metric.path, metric]));
    this.previousMetrics = focusPaths.map((path) => byPath.get(path)).filter(Boolean);
    this.previousTopics = this.previousMetrics.map((metric) => label(metric.name));
    this.recentMetrics = recentPaths.map((path) => byPath.get(path)).filter(Boolean);
  }

  /** @param {(status: string) => void} onStatus @param {AbortSignal} signal */
  async tryDirect(onStatus, signal) {
    if (this.directApiHint) {
      this.outcome = "read_api";
      this.queries = [this.request];
      this.apiOptions = [{
        ref: this.refs.issue("api", this.directApiHint, this.directApiHint.key),
        label: `${this.directApiHint.method} ${this.directApiHint.path} — ${this.directApiHint.label}`,
        operation: this.directApiHint,
      }];
      try {
        const direct = await this.tryDirectApi(onStatus, signal);
        if (direct) return direct;
      } catch {
        this.requiresTools = true;
        this.apiOptions = [];
      }
    }

    if (this.apiCandidates.length) {
      this.outcome = "read_api";
      this.queries = [this.request];
      await this.searchApiOperations(onStatus);
      return undefined;
    }

    const evidence = directEvidenceFocus(this.request);
    const chart = evidence
      ? undefined
      : directChartCommand(this.request, Boolean(this.activeChart));
    if (chart) {
      let metrics = await mentionedMetrics(
        this.request,
        () => onStatus("Indexing metrics…"),
      );
      if (metrics.length < 2 && mayRequestMultiple(this.request)) {
        metrics = await coordinatedMetrics(
          this.request,
          () => onStatus("Searching metrics…"),
        );
      }
      if (!metrics.length) {
        if (this.previousTopics.length > 1 && referencesSingular(this.request)) {
          return undefined;
        }
        metrics = referencesPlural(this.request) && this.recentMetrics.length
          ? this.recentMetrics
          : this.previousMetrics.length
          ? this.previousMetrics
          : await exactTopicMetrics(
              this.previousTopics,
              () => onStatus("Indexing metrics…"),
            );
      }

      if (metrics.length && (!mayRequestMultiple(this.request) || metrics.length > 1)) {
        const refs = metrics.map((metric) => this.refs.issue("metric", metric, metric.path));
        this.outcome = chart.kind === "edit"
          ? "edit_existing_chart"
          : "build_requested_chart";
        onStatus("Building chart…");
        const result = this.buildChart({ refs, operation: chart.operation });
        return { output: result.output, artifacts: result.artifacts };
      }
      this.requiresTools = true;
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
      if (evidence === "implementation") {
        onStatus("Searching source…");
        const result = await this.source.search(
          this.request,
          undefined,
          ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
        );
        if (result.matches.length) {
          const options = result.matches.slice(0, 6).map((/** @type {any} */ match) => {
            const value = { ...match, revision: result.revision };
            return {
              ref: this.refs.issue("source", value, `${match.path}:${match.startLine}`),
              kind: "source",
              label: `${match.path}:${match.startLine}`,
              detail: match.content.slice(0, 180),
            };
          });
          this.outcome = "explain_from_verified_facts";
          this.stage = "resolve";
          this.options = options;
          this.observation = { options };
          this.requiresTools = true;
          return undefined;
        }
      }
      this.requiresTools = true;
    }

    if (
      isDirectValueRequest(this.request) ||
      this.previousTopics.length === 1 && isDirectValueFollowup(this.request)
    ) {
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
        this.requiresTools = true;
      }
    }

    if (!isDirectDefinition(this.request)) return undefined;

    const explicitlyNamesMetric = /\b(?:indicator|metric|series)\b/i.test(this.request);
    const mentioned = await mentionedMetrics(
      this.request,
      () => onStatus("Indexing metrics…"),
    );
    if (mentioned.length > 1) return undefined;
    let [metric] = mentioned;
    if (!metric && this.previousMetrics.length === 1 && referencesPrevious(this.request)) {
      [metric] = this.previousMetrics;
    }
    if (!metric && !explicitlyNamesMetric) return undefined;
    if (!metric) {
      [metric] = await searchMetrics(
        [this.request],
        1,
        [],
        () => onStatus("Indexing metrics…"),
      );
    }
    if (!metric) {
      this.requiresTools = true;
      return undefined;
    }

    onStatus("Searching source…");
    const formula = await this.source.explain(
      `${this.request}\n${metric.name}`,
      ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
    );
    if (!formula || normalize(formula.fact.metric) !== normalize(metric.name)) {
      this.requiresTools = true;
      return undefined;
    }

    this.rememberMetricValues([metric]);
    return {
      output: renderEvidence({
        facts: [formula.answer],
        sources: [formulaCitation(formula)],
        excerpts: [],
      }),
      artifacts: [],
    };
  }

  async tool() {
    if (this.stage === "search") {
      return searchTool(
        Boolean(this.activeChart),
        this.previousTopics.length > 0 || Boolean(this.previousApi),
      );
    }
    if (this.stage === "rewrite") return rewriteTool(this.queries);
    if (this.stage === "resolve") {
      if (this.outcome === "read_api") return apiResolveTool(this.apiOptions);
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
      const previousApi = this.previousApi
        ? `\nPrevious verified API resource: ${this.previousApi.operation.label} (${this.previousApi.operation.key}), arguments ${JSON.stringify(this.previousApi.arguments)}. Reuse only for a dependent follow-up on that resource.`
        : "";
      const chart = this.activeChart
        ? `\nActive chart: ${this.activeChart.chart.title}; series: ${this.activeChart.chart.series.map((item) => item.label).join(", ")}.`
        : "";
      return `${ASK_STAGE_PROMPTS.search}${previous}${previousApi}${chart}`;
    }
    if (this.stage === "rewrite") return ASK_STAGE_PROMPTS.rewrite;
    if (this.stage === "resolve") {
      if (this.outcome === "read_api") return ASK_STAGE_PROMPTS.api;
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
    if (this.outcome === "read_api") return await this.searchApiOperations(onStatus);

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

    const hasExactMetric = foundMetrics.some((metric) =>
      normalize(metric.name) === normalize(metric.matchedQuery ?? "")
    );
    const hasGroundedSubject = hasExactMetric ||
      this.contextMetrics.length > 0 ||
      this.rewritten && this.rewriteChanged;

    if (
      this.outcome === "explain_from_verified_facts" &&
      this.focus !== "variants" &&
      !this.sourceSearched &&
      (this.focus === "implementation" || hasGroundedSubject)
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
        const result = await this.source.search(
          this.queries.join(" "),
          undefined,
          ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
        );
        sources = result.matches.map((/** @type {any} */ source) => ({
          ...source,
          revision: result.revision,
        }));
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

    const normalizedQueries = this.queries.map((query) => normalize(canonicalMetricQuery(query)));
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
      Boolean(this.formula) ||
      sources.length > 0;
    if (!trusted) {
      if (!this.rewritten) {
        this.stage = "rewrite";
        this.observation = { unmatchedQueries: this.queries };
        return;
      }
      return {
        done: true,
        output: "I couldn't find a matching series. What metric should I use instead?",
      };
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

  /** @param {(status: string) => void} onStatus */
  async searchApiOperations(onStatus) {
    onStatus("Searching API…");
    const values = literalArguments(this.request);
    const found = await searchApi(
      [this.request, ...this.queries],
      MAX_OPTIONS,
      () => onStatus("Indexing API…"),
    );
    const candidates = [...new Map(
      [...this.apiHints, ...found].map((operation) => [operation.key, operation]),
    ).values()];
    candidates.sort((left, right) => {
      const leftRequired = left.parameters.filter((parameter) => parameter.required).length;
      const rightRequired = right.parameters.filter((parameter) => parameter.required).length;
      return argumentAffinity(values, right, this.request) -
          argumentAffinity(values, left, this.request) ||
        Number(rightRequired === values.length) - Number(leftRequired === values.length) ||
        (right.score ?? 0) - (left.score ?? 0);
    });
    const operations = this.reuseApiContext && this.previousApi
      ? [
          this.previousApi.operation,
          ...candidates.filter((operation) => operation.key !== this.previousApi?.operation.key),
        ]
      : candidates;
    this.apiOptions = operations.slice(0, MAX_API_OPTIONS).map((operation) => ({
      ref: this.refs.issue("api", operation, operation.key),
      label: `${operation.method} ${operation.path} — ${operation.label}`,
      operation,
    }));
    if (!this.apiOptions.length) {
      this.stage = "clarify";
      this.observation = { noApiMatch: true };
      return;
    }
    this.stage = "resolve";
    this.observation = {
      verifiedOperations: this.apiOptions.map(({ ref, operation }) => ({
        ref,
        method: operation.method,
        path: operation.path,
        summary: operation.summary || operation.label,
        description: operation.description,
        parameters: operation.parameters,
        response: {
          type: operation.response.type,
          description: operation.response.description,
          fields: operation.response.fields
            .slice(0, 12)
            .map(({ name, type, description }) => ({ name, type, description })),
        },
      })),
    };
  }

  /** @param {(status: string) => void} onStatus @param {AbortSignal} signal */
  async tryDirectApi(onStatus, signal) {
    const operation = this.apiOptions[0]?.operation;
    if (!operation) return undefined;
    const required = operation.parameters.filter((parameter) => parameter.required);
    const values = literalArguments(this.request);
    if (!values.length) {
      const arguments_ = reusableArguments(operation, this.previousApi);
      if (arguments_ === undefined && required.length) return undefined;
      return await this.callApi(
        operation,
        arguments_ ?? {},
        onStatus,
        signal,
      );
    }
    if (values.length !== required.length) return undefined;
    const arguments_ = Object.fromEntries(
      required.map((parameter, index) => [parameter.name, values[index]]),
    );
    return await this.callApi(operation, arguments_, onStatus, signal);
  }

  /**
   * @param {import("./api/index.js").ApiOperation} operation
   * @param {Record<string, unknown>} arguments_
   * @param {(status: string) => void} onStatus
   * @param {AbortSignal} signal
   */
  async callApi(operation, arguments_, onStatus, signal) {
    onStatus("Reading API…");
    const result = await executeApi(operation, arguments_, signal);
    this.previousApi = { operation, arguments: result.arguments };
    return {
      done: true,
      apiGrounding: {
        question: this.request,
        queries: this.queries,
        ...result,
      },
    };
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
      sources: /** @type {{ revision: string, path: string, startLine: number, endLine?: number }[]} */ ([]),
      excerpts: /** @type {{ revision: string, path: string, startLine: number, endLine?: number, content: string }[]} */ ([]),
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
        evidence.sources.push(formulaCitation(fact));
      } else if (kind === "source") {
        const match = this.refs.get(ref, "source");
        const excerpt = await this.source.read(match.path, match.startLine, match.endLine);
        evidence.excerpts.push(excerpt);
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
      evidence.sources.push(formulaCitation(this.formula));
    }

    if (rememberedMetrics.length) {
      this.rememberMetricValues(rememberedMetrics);
    } else {
      this.previousMetrics = [];
      this.previousTopics = [...new Set(topics.length ? topics : this.queries)].slice(0, 4);
    }
    return {
      done: true,
      output: renderEvidence(evidence),
      ...(evidence.excerpts.length
        ? {
            grounding: {
              question: this.request,
              excerpts: evidence.excerpts,
            },
          }
        : {}),
    };
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

    const { unit, conflicts } = resolveChartUnit(
      chosen,
      existingChart?.chart.unit,
      operation,
    );
    if (conflicts.length) {
      return {
        done: true,
        output: `Those metrics use different units (${conflicts.map((value) => `**${value.toUpperCase()}**`).join(" and ")}), so they need separate charts. Which one should I chart?`,
        artifacts: [],
      };
    }
    const artifact = createChartArtifact({
      title: typeof action.title === "string" && action.title.trim()
        ? action.title.trim()
        : series.map((item) => item.label).join(" and "),
      unit,
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
  /** @param {Record<string, unknown>} action @param {(status: string) => void} onStatus @param {AbortSignal} signal */
  async execute(action, onStatus, signal) {
    const name = requiredString(action.action, "action");
    if (name === "clarify") {
      const text = typeof action.text === "string" ? action.text.trim() : "";
      return {
        done: true,
        output: text || "I couldn't find a matching series. Which metric should I use instead?",
      };
    }
    if (this.stage === "search") {
      if (name !== "search") throw new Error("The AI chose an invalid search action");
      this.outcome = requiredString(action.outcome, "outcome");
      if (this.outcome === "clarify_request") {
        return {
          done: true,
          output: requiredString(action.clarification, "clarification"),
        };
      }
      if (this.outcome === "answer_general") {
        return { done: true, general: true };
      }
      const context = typeof action.context === "string" && action.context.trim()
        ? action.context.trim()
        : (this.previousTopics.length || this.previousApi) && referencesPrevious(this.request)
          ? "reuse_previous"
          : "new_topic";
      const proposed = Array.isArray(action.queries) && action.queries.length
        ? requiredStrings(action.queries, "queries")
        : [this.request];
      const cardinality = typeof action.cardinality === "string"
        ? action.cardinality
        : "single";
      const effectiveContext = context;
      if (
        effectiveContext !== "new_topic" &&
        !this.previousTopics.length &&
        !this.previousApi
      ) {
        throw new Error("There is no previous verified topic to reuse");
      }
      const previousTopics = [...this.previousTopics];
      const previousMetrics = [...this.previousMetrics];
      const routedQueries = this.outcome === "read_api"
        ? proposed
        : effectiveContext === "reuse_previous"
          ? previousTopics
          : effectiveContext === "extend_previous"
            ? [...new Set([...previousTopics, ...proposed])]
            : proposed;
      this.contextMetrics = effectiveContext === "new_topic" ? [] : previousMetrics;
      this.reuseApiContext = effectiveContext !== "new_topic" && Boolean(this.previousApi);
      if (effectiveContext === "new_topic") {
        this.previousMetrics = [];
        this.previousTopics = [];
        this.previousApi = undefined;
      }
      this.focus = evidenceFocus(this.request, "definition");
      this.comparison = cardinality === "multiple" || isExplicitComparison(this.request);
      this.queries = this.comparison
        ? completeComparisonQueries(routedQueries)
        : routedQueries.slice(0, 1);
      this.query = this.queries.join(" / ");
      const searched = await this.search(onStatus);
      if (this.outcome === "read_api" && this.apiOptions.length) {
        const direct = await this.tryDirectApi(onStatus, signal);
        if (direct) return direct;
      }
      return searched ?? { done: false };
    }
    if (this.stage === "resolve" && this.outcome === "explain_from_verified_facts") {
      if (name !== "answer") throw new Error("The AI chose an invalid evidence action");
      onStatus("Inspecting results…");
      return this.inspect(uniqueRefs(action.refs));
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
    if (this.stage === "resolve" && this.outcome === "read_api") {
      if (name !== "call_api") throw new Error("The AI chose an invalid API action");
      const ref = requiredString(action.ref, "ref");
      const operation = this.refs.get(ref, "api");
      const supplied = action.arguments && typeof action.arguments === "object"
        ? /** @type {Record<string, unknown>} */ (action.arguments)
        : {};
      const previousApi = this.previousApi;
      const arguments_ = previousApi && previousApi.operation.key === operation.key
        ? { ...previousApi.arguments, ...supplied }
        : supplied;
      return await this.callApi(operation, arguments_, onStatus, signal);
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
    this.recentMetrics = [...new Map(
      [...this.previousMetrics, ...this.recentMetrics]
        .map((metric) => [metric.path, metric]),
    ).values()].slice(0, 6);
  }

  metricPaths() {
    return this.previousMetrics.map((metric) => metric.path);
  }

  apiContext() {
    return this.previousApi
      ? {
          key: this.previousApi.operation.key,
          arguments: this.previousApi.arguments,
        }
      : undefined;
  }
}
