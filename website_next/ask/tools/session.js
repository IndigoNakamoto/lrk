import { createChartArtifact } from "./chart.js";
import { resolveChartUnit } from "./chart/units.js";
import { apiByKey, searchApi } from "./api/index.js";
import { executeApi } from "./api/execute.js";
import {
  apiResponseMatchCount,
  apiOperationMatchCount,
  argumentAffinity,
  directApiCandidate,
  literalArguments,
  literalType,
  matchesApiIntent,
  matchesApiResponse,
  reusableArguments,
} from "./api/routing.js";
import { readMetric } from "./data.js";
import { directChartCommand } from "./direct/chart.js";
import { directEvidenceFocus } from "./direct/evidence.js";
import {
  completeComparisonQueries,
  directReadAction,
  evidenceFocus,
  isDirectDefinition,
  isDirectValueFollowup,
  isDirectValueRequest,
  isExplicitComparison,
  mayRequestMultiple,
  referencesPlural,
  referencesPrevious,
  referencesSingular,
} from "./direct/request.js";
import {
  latestApiContext,
  latestChart,
  latestKnowledgeContext,
  latestMetricPaths,
  latestSourceContext,
  recentMetricPaths,
} from "./history.js";
import { searchLearn } from "./learn.js";
import {
  metricByName,
  metricVariants,
  metricsByPaths,
  mentionedMetrics,
  searchMetrics,
} from "./metrics/index.js";
import {
  balancedOptions,
  coordinatedMetrics,
  exactTopicMetrics,
  MAX_OPTIONS,
  mergeMetricGroups,
  uniqueMetricOptions,
} from "./metrics/options.js";
import { canonicalMetricQuery } from "./metrics/language.js";
import { ASK_STAGE_PROMPTS } from "./prompts.js";
import { AskRefs } from "./refs.js";
import { renderData, renderEvidence } from "./render.js";
import {
  directSourceFact,
  hasDirectSourceComputation,
  sourceFocus,
} from "./source/answer.js";
import {
  apiResolveTool,
  clarifyTool,
  resolveTool,
  rewriteTool,
  searchTool,
} from "./schemas.js";
import { normalize } from "./text.js";

const MAX_API_OPTIONS = 6;
const MAX_API_HINTS = 24;

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

/** @param {string} formulaMetric @param {string} metricName */
function formulaMatchesMetric(formulaMetric, metricName) {
  const formula = normalize(formulaMetric);
  const metric = normalize(metricName);
  return metric === formula || metric.endsWith(` ${formula}`);
}

/** @param {any} formula @param {any} metric */
function metricFormulaAnswer(formula, metric) {
  if (!formula.fact.summary) return formula.answer;
  const name = label(metric.name);
  const displayName = !name.includes(" ") && name.length <= 4
    ? name.toUpperCase()
    : `${name[0].toUpperCase()}${name.slice(1)}`;
  const unit = metric.suggestedUnit && metric.suggestedUnit !== "number"
    ? ` (${String(metric.suggestedUnit).toUpperCase()})`
    : "";
  return `${displayName}${unit} ${formula.fact.summary}.`;
}

/** @param {any} formula */
function formulaCitation(formula) {
  return {
    revision: formula.revision,
    path: formula.fact.path,
    startLine: formula.fact.line,
  };
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
  verifyDirectApiIntent = false;
  /** @type {{ operation: import("./api/index.js").ApiOperation, arguments: Record<string, unknown> } | undefined} */
  previousApi;
  reuseApiContext = false;
  /** @type {any} */
  formula;
  sourceSearched = false;
  /** @type {import("../storage.js").SourceContext[]} */
  previousSource = [];
  /** @type {import("../storage.js").KnowledgeContext | undefined} */
  previousKnowledge;
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
    this.verifyDirectApiIntent = false;
    this.reuseApiContext = false;
    this.formula = undefined;
    this.sourceSearched = false;
    this.previousSource = latestSourceContext(history) ?? [];
    this.previousKnowledge = latestKnowledgeContext(history);
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

    const focusPaths = latestMetricPaths(history);
    const literals = literalArguments(this.request);
    const hasResourceIdentifier = literals.some((value) => literalType(value) === "string");
    const dependsOnMetric = Boolean(focusPaths?.length) &&
      !this.previousApi &&
      referencesPrevious(this.request) &&
      !hasResourceIdentifier;
    let prefersMetricTool = dependsOnMetric ||
      Boolean(directChartCommand(this.request, Boolean(this.activeChart)));
    const dependsOnPrevious = Boolean(this.previousApi) && referencesPrevious(this.request);
    const previousApiContext = this.previousApi
      ? [
          this.previousApi.operation.summary || this.previousApi.operation.label,
          ...this.previousApi.operation.parameters.flatMap((parameter) => [
            parameter.name,
            parameter.type,
            parameter.description,
          ]),
        ].filter(Boolean).join(" ")
      : "";
    const apiQuery = dependsOnPrevious
      ? `${this.request} ${previousApiContext}`
      : this.request;
    this.apiHints = await searchApi([apiQuery], MAX_API_HINTS, onProgress).catch(() => []);
    this.apiCandidates = !prefersMetricTool && literals.length
      ? this.apiHints.filter((operation) =>
          argumentAffinity(literals, operation, this.request) >= 0 &&
          (operation.matchedTerms ?? 0) > 0
        )
      : [];
    if (
      !prefersMetricTool &&
      !this.previousApi &&
      isDirectValueRequest(this.request) &&
      literals.some((value) => literalType(value) !== "string")
    ) {
      const pointMetrics = await mentionedMetrics(this.request, onProgress);
      if (pointMetrics.length) {
        prefersMetricTool = true;
        this.apiCandidates = [];
      }
    }
    this.directApiHint = prefersMetricTool
      ? undefined
      : directApiCandidate(this.apiHints, literals, this.request);
    this.verifyDirectApiIntent = Boolean(
      this.directApiHint &&
        !this.previousApi &&
        !literals.length &&
        this.apiHints[0]?.key !== this.directApiHint.key,
    );
    this.requiresTools = Boolean(this.directApiHint) || this.apiCandidates.length > 0;

    if (
      !prefersMetricTool &&
      this.previousApi &&
      (
        referencesPrevious(this.request) ||
        matchesApiResponse(this.request, this.previousApi.operation)
      )
    ) {
      this.requiresTools = true;
      const contextual = this.apiHints
        .filter((operation) =>
          (operation.matchedTerms ?? 0) >= 2 &&
          matchesApiIntent(this.request, operation) &&
          reusableArguments(operation, this.previousApi) !== undefined
        )
        .sort((left, right) =>
          apiResponseMatchCount(this.request, right) -
            apiResponseMatchCount(this.request, left) ||
          (right.matchedTerms ?? 0) - (left.matchedTerms ?? 0) ||
          (right.score ?? 0) - (left.score ?? 0)
        )[0];
      if (contextual) {
        this.directApiHint = contextual;
      } else if (matchesApiResponse(this.request, this.previousApi.operation)) {
        this.directApiHint = this.previousApi.operation;
      }
    }

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
      if (chart.kind === "missing") {
        return {
          output: "Which metric should I chart?",
          artifacts: [],
        };
      }
      if (chart.kind === "missing_add") {
        const added = await mentionedMetrics(
          this.request,
          () => onStatus("Indexing metrics…"),
        );
        const metrics = [...new Map(
          [...this.previousMetrics, ...added].map((metric) => [metric.path, metric]),
        ).values()];
        if (metrics.length > 1) {
          this.rememberMetricValues(metrics);
          return {
            output: `Do you want me to chart ${metrics.map((metric) => label(metric.name)).join(" and ")} together?`,
            artifacts: [],
          };
        }
        return {
          output: "What should I add it to?",
          artifacts: [],
        };
      }
      if (chart.kind === "style") {
        this.outcome = "edit_existing_chart";
        onStatus("Updating chart…");
        const result = this.restyleChart(chart);
        return { output: result.output, artifacts: result.artifacts };
      }

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
        try {
          const result = this.buildChart({ refs, operation: chart.operation });
          return { output: result.output, artifacts: result.artifacts };
        } catch (error) {
          if (chart.kind !== "edit") throw error;
          return {
            output: error instanceof Error ? error.message : String(error),
            artifacts: [],
          };
        }
      }
      this.requiresTools = true;
    }

    if (evidence) {
      this.requiresTools = true;
      let metrics = await mentionedMetrics(
        this.request,
        () => onStatus("Indexing metrics…"),
      );
      if (!metrics.length && this.previousMetrics.length === 1 && referencesPrevious(this.request)) {
        metrics = this.previousMetrics;
      }

      if (metrics.length) {
        if (evidence === "implementation" && metrics.length !== 1) {
          metrics = [];
        }
        if (evidence === "variants") {
          const supported = await Promise.all(
            metrics.map(async (metric) =>
              await metricVariants(metric, this.request) ? metric : undefined
            ),
          );
          metrics = supported.filter((metric) => metric !== undefined);
          if (!metrics.length) return undefined;
        }
        if (evidence === "implementation" && metrics.length) {
          onStatus("Searching source…");
          this.formula = await this.source.explain(
            [this.request, ...metrics.map((metric) => metric.name)].join("\n"),
            ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
          );
          if (
            !this.formula ||
            !metrics.some((metric) =>
              formulaMatchesMetric(this.formula.fact.metric, metric.name)
            )
          ) {
            metrics = [];
          }
        }

        if (metrics.length) {
          this.focus = evidence;
          const refs = metrics.map((metric) => this.refs.issue("metric", metric, metric.path));
          onStatus("Inspecting results…");
          const result = await this.inspect(refs);
          return { output: result.output, artifacts: [] };
        }
      }
      if (evidence === "implementation") {
        if (
          this.previousSource.length &&
          directSourceFact(this.request, this.previousSource)
        ) {
          return {
            output: "",
            artifacts: [],
            grounding: {
              question: this.request,
              excerpts: this.previousSource,
            },
          };
        }
        onStatus("Searching source…");
        const result = await this.source.search(
          this.request,
          undefined,
          ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
        );
        if (result.matches.length) {
          const [match] = result.matches;
          const excerpt = { ...match, revision: result.revision };
          if (hasDirectSourceComputation(this.request, [excerpt])) {
            const focus = sourceFocus(this.request, [excerpt]);
            return {
              output: "",
              artifacts: [],
              grounding: {
                question: this.request,
                excerpts: [{ ...excerpt, ...(focus ? { focus } : {}) }],
              },
            };
          }
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
      this.previousSource.length &&
      (
        directSourceFact(this.request, this.previousSource) ||
        referencesPrevious(this.request) &&
          /^(?:are|does|how|is|what|which|why)\b/i.test(this.request.trim())
      ) &&
      !isDirectValueRequest(this.request)
    ) {
      this.requiresTools = true;
      return {
        output: "",
        artifacts: [],
        grounding: {
          question: this.request,
          excerpts: this.previousSource,
        },
      };
    }

    if (isExplicitComparison(this.request)) {
      let metrics = await mentionedMetrics(
        this.request,
        () => onStatus("Indexing metrics…"),
      );
      if (referencesPrevious(this.request)) {
        metrics = [...this.previousMetrics, ...metrics];
      }
      metrics = [...new Map(metrics.map((metric) => [metric.path, metric])).values()];
      if (metrics.length >= 2) {
        onStatus("Searching source…");
        const formulas = (
          await Promise.all(metrics.map((metric) =>
            this.source.explain(
              `${this.request}\n${metric.name}`,
              ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
            )
          ))
        ).filter(Boolean);
        if (formulas.length) {
          const facts = formulas.map((formula) => {
            const metric = metrics.find((item) =>
              formulaMatchesMetric(formula.fact.metric, item.name)
            );
            return metric ? metricFormulaAnswer(formula, metric) : formula.answer;
          });
          for (const formula of formulas) {
            if (formula.fact.kind !== "ratio") continue;
            const denominator = normalize(formula.fact.denominator ?? "");
            const related = metrics.find((metric) =>
              normalize(metric.name) === denominator ||
              normalize(metric.name).endsWith(` ${denominator}`)
            );
            if (related?.suggestedUnit) {
              facts.push(
                `${label(related.name)} is the ${String(related.suggestedUnit).toUpperCase()} denominator in ${formula.fact.metric.toUpperCase()}.`,
              );
            }
          }
          this.rememberMetricValues(metrics);
          return {
            output: renderEvidence({
              facts,
              sources: formulas.map(formulaCitation),
              excerpts: [],
            }),
            artifacts: [],
          };
        }
        this.rememberMetricValues(metrics);
        return {
          output: "Do you want their latest values, definitions, or a chart?",
          artifacts: [],
        };
      }
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

        if (metrics.length > 1 && referencesSingular(this.request)) {
          this.requiresTools = true;
          return undefined;
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

    if (mayRequestMultiple(this.request)) {
      const metrics = await mentionedMetrics(
        this.request,
        () => onStatus("Indexing metrics…"),
      );
      if (metrics.length >= 2) {
        this.rememberMetricValues(metrics);
        return {
          output: "Do you want their latest values, definitions, or a chart?",
          artifacts: [],
        };
      }
    }

    if (
      this.previousKnowledge &&
      (
        referencesPrevious(this.request) ||
        /\b(?:analogy|trade-?off)\b/i.test(this.request)
      )
    ) {
      if (/\btrade-?off\b/i.test(this.request)) {
        return {
          output: `Which tradeoff about ${this.previousKnowledge.title} do you mean?`,
          artifacts: [],
          knowledgeContext: this.previousKnowledge,
        };
      }
      return {
        output: "",
        artifacts: [],
        knowledgeGrounding: {
          question: this.request,
          context: this.previousKnowledge,
        },
      };
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
    if (!metric && !explicitlyNamesMetric) {
      const [guide] = await searchLearn(this.request, 1);
      if (guide?.titleCoverage === 1 && guide.description) {
        const guideMetrics = (
          await Promise.all(
            guide.series.map((/** @type {any} */ series) => metricByName(series.name)),
          )
        ).filter(Boolean);
        if (guideMetrics.length) this.rememberMetricValues(guideMetrics);
        return {
          output: guide.description,
          artifacts: [],
          knowledgeContext: {
            title: guide.title,
            description: guide.description,
          },
        };
      }
      return undefined;
    }
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
    if (!formula || !formulaMatchesMetric(formula.fact.metric, metric.name)) {
      const result = await this.source.search(
        `${metric.name} compute calculation definition state`,
        undefined,
        ({ loaded, total }) => onStatus(`Indexing source · ${loaded} / ${total}`),
      );
      const [match] = result.matches;
      if (!match) {
        this.requiresTools = true;
        return undefined;
      }

      onStatus("Inspecting results…");
      const excerpt = await this.source.read(
        match.path,
        match.startLine,
        match.endLine,
      );
      this.rememberMetricValues([metric]);
      return {
        output: "",
        artifacts: [],
        grounding: {
          question: this.request,
          metric: {
            name: label(metric.name),
            path: metric.path,
            unit: metric.suggestedUnit,
          },
          excerpts: [excerpt],
        },
      };
    }

    this.rememberMetricValues([metric]);
    return {
      output: renderEvidence({
        facts: [metricFormulaAnswer(formula, metric)],
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
      return apiOperationMatchCount(this.request, right) -
          apiOperationMatchCount(this.request, left) ||
        argumentAffinity(values, right, this.request) -
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
    if (
      isExplicitComparison(this.request) &&
      this.previousApi?.operation.key === operation.key &&
      required.every((parameter) =>
        Object.hasOwn(this.previousApi?.arguments ?? {}, parameter.name)
      )
    ) {
      onStatus("Comparing API data…");
      const [previous, current] = await Promise.all([
        executeApi(operation, this.previousApi.arguments, signal),
        executeApi(operation, arguments_, signal),
      ]);
      this.previousApi = { operation, arguments: current.arguments };
      return {
        done: true,
        apiGroundings: [previous, current].map((result) => ({
          question: this.request,
          queries: this.queries,
          ...result,
        })),
      };
    }
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
    this.previousMetrics = [];
    this.previousTopics = [];
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
        const focus = sourceFocus(this.request, [excerpt]);
        evidence.excerpts.push({ ...excerpt, ...(focus ? { focus } : {}) });
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
        output: `Those metrics use different units (${conflicts.map((value) => value === "number" ? "**unitless**" : `**${value.toUpperCase()}**`).join(" and ")}), so they need separate charts. Which one should I chart?`,
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

  /** @param {{ scale?: string, view?: string }} action */
  restyleChart(action) {
    if (!this.activeChart) throw new Error("There is no chart to update");
    const current = this.activeChart.chart;
    const artifact = createChartArtifact({
      title: current.title,
      unit: current.unit,
      view: action.view ?? current.view,
      scale: action.scale ?? current.scale,
      series: current.series,
    });
    this.activeChart = artifact;
    const changes = [
      action.view ? `${action.view} view` : "",
      action.scale ? `${action.scale} scale` : "",
    ].filter(Boolean).join(" and ");
    return {
      done: true,
      output: `Updated **${artifact.chart.title}** to use ${changes}.`,
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
    this.previousApi = undefined;
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
