import { AskRefs } from "../refs.js";
import {
  ROUTE_INSTRUCTION,
  actionInstruction,
  actionTool,
  apiArgumentTool,
  availableActions,
  capabilityMetrics,
  directAction,
  generalCapabilities,
  routeTools,
} from "./capabilities.js";
import { loadSessionContext } from "./context.js";
import { collectEvidence, collectSourceOptions } from "./evidence.js";
import { CapabilityExecutor } from "./executor.js";
import { normalize } from "../text.js";
import {
  explicitArguments,
  hasRequiredArguments,
  reusableArguments,
  validatedArguments,
} from "../api/routing.js";

/** @param {unknown[]} values */
function schemaTokens(values) {
  return new Set(
    normalize(values.filter(Boolean).join(" "))
      .split(" ")
      .filter((token) => token.length >= 3),
  );
}

/** @param {Set<string>} query @param {Set<string>} document */
function overlaps(query, document) {
  return [...query].some((token) => document.has(token));
}

export class AskToolSession {
  /** @param {import("../source/index.js").AskSource} source */
  constructor(source) {
    this.source = source;
  }

  /** @param {string} question @param {import("../../storage.js").StoredMessage[]} history @param {(status: string) => void} onStatus */
  async begin(question, history, onStatus) {
    const refs = new AskRefs();
    this.refs = refs;
    const context = await loadSessionContext(
      history,
      () => onStatus("Indexing context…"),
    );
    onStatus("Searching tools…");
    const evidence = await collectEvidence({
      question,
      context,
      refs,
      onStatus,
    });
    this.question = question;
    this.evidence = evidence;
    this.executor = new CapabilityExecutor({
      question,
      evidence,
      refs,
      source: this.source,
    });
  }

  question = "";
  /** @type {Awaited<ReturnType<typeof collectEvidence>> | undefined} */
  evidence;
  /** @type {CapabilityExecutor | undefined} */
  executor;
  /** @type {AskRefs | undefined} */
  refs;

  routeTools() {
    if (!this.evidence) throw new Error("Tool session is not ready");
    return routeTools(this.evidence);
  }

  hasSourceContext() {
    return Boolean(this.evidence?.context.source.length);
  }

  routeMessages() {
    if (!this.evidence) throw new Error("Tool session is not ready");
    const { context, metricOptions, apiOptions, sourceOptions, guideOptions } =
      this.evidence;
    return [
      {
        role: /** @type {const} */ ("system"),
        content: ROUTE_INSTRUCTION,
      },
      {
        role: /** @type {const} */ ("user"),
        content: JSON.stringify({
          request: this.question,
          context: {
            activeCapability: context.chart
              ? "chart"
              : context.api
                ? "api"
                : context.metrics.length
                  ? "metric"
                  : context.source.length
                    ? "source"
                    : context.knowledge
                      ? "general"
                      : undefined,
            ...(context.chart
              ? {
                  activeChart: {
                    title: context.chart.chart.title,
                    series: context.chart.chart.series.map(
                      (/** @type {any} */ { path, label }) => ({ path, label }),
                    ),
                  },
                }
              : {}),
            ...(context.metrics.length
              ? {
                  metrics: context.metrics.map(
                    (/** @type {any} */ metric) => ({
                      name: metric.name,
                      path: metric.path,
                    }),
                  ),
                }
              : {}),
            ...(context.recentMetrics.length
              ? {
                  recentMetrics: context.recentMetrics.map(
                    (/** @type {any} */ metric) => ({
                      name: metric.name,
                      path: metric.path,
                    }),
                  ),
                }
              : {}),
            ...(context.api
              ? {
                  api: {
                    operation: context.api.operation.label,
                    arguments: context.api.arguments,
                  },
                }
              : {}),
            ...(context.source.length
              ? {
                  source: context.source.map(
                    (/** @type {any} */ source) => source.path,
                  ),
                }
              : {}),
            ...(context.knowledge ? { concept: context.knowledge } : {}),
          },
          matches: {
            metrics: metricOptions.map(({ label, origin }) => ({
              label,
              origin,
            })),
            api: apiOptions.map(({ label, operation }) => ({
              label,
              required: operation.parameters
                .filter((/** @type {any} */ parameter) => parameter.required)
                .map((/** @type {any} */ parameter) => ({
                  name: parameter.name,
                  type: parameter.valueType || parameter.type,
                  description: parameter.description,
                })),
              returns: operation.response.fields
                .slice(0, 8)
                .map((/** @type {any} */ field) => field.name),
            })),
            source: sourceOptions.map((/** @type {any} */ { ref, source }) => ({
              ref,
              path: source.path,
            })),
            guides: guideOptions.map(({ label }) => label),
          },
        }),
      },
    ];
  }

  directRoute() {
    if (!this.evidence) return undefined;
    const query = schemaTokens([this.question]);
    const contextKey = this.evidence.context.api?.operation.key;
    for (const { ref, operation } of this.evidence.apiOptions) {
      const required = schemaTokens(
        operation.parameters
          .filter((/** @type {any} */ parameter) => parameter.required)
          .flatMap((/** @type {any} */ parameter) => [
            parameter.name,
            parameter.description,
          ]),
      );
      const returned = schemaTokens(
        operation.response.fields.flatMap((/** @type {any} */ field) => [
          field.name,
          field.description,
        ]),
      );
      const fieldMatch = overlaps(query, returned);
      const suppliedResource = required.size > 0 && overlaps(query, required);
      if (
        fieldMatch &&
        (suppliedResource || operation.key === contextKey)
      ) {
        return {
          action: "call_api",
          call: {
            name: "call_api",
            arguments: { ref },
          },
        };
      }
    }
    const action = directAction(this.evidence, this.question);
    if (action === "search_source") {
      return {
        action,
        call: {
          name: action,
          arguments: { query: this.question },
        },
      };
    }
    if (action) {
      return {
        action,
        call: this.directCall(action),
      };
    }
    return undefined;
  }

  /** @param {string} action @param {(status: string) => void} onStatus */
  async prepareAction(action, onStatus) {
    if (
      action !== "explain_evidence" ||
      !this.evidence ||
      !this.refs
    ) return;
    this.evidence.sourceOptions = await collectSourceOptions({
      question: this.question,
      evidence: this.evidence,
      source: this.source,
      refs: this.refs,
      onStatus,
    });
  }

  /** @param {string} action */
  actionTool(action) {
    const evidence = this.evidence;
    if (!evidence) throw new Error("Tool session is not ready");
    if (!availableActions(evidence).includes(action)) {
      throw new Error(`Unavailable AI capability: ${action}`);
    }
    return actionTool(evidence, action);
  }

  /** @param {string} action */
  directCall(action) {
    const evidence = this.evidence;
    if (!evidence) return undefined;

    const metrics = capabilityMetrics(evidence, action);
    if (action === "describe_capabilities") {
      return {
        name: action,
        arguments: { capabilities: generalCapabilities() },
      };
    }
    if (
      action === "add_chart_series" ||
      action === "remove_chart_series" ||
      action === "replace_chart_series"
    ) {
      const activePaths = new Set(
        evidence.context.chart?.chart.series.map(
          (/** @type {{ path: string }} */ { path }) => path,
        ) ?? [],
      );
      const mentioned = evidence.metricOptions.filter(
        ({ metric, origin }) =>
          origin === "mentioned" &&
          (
            action === "add_chart_series"
              ? !activePaths.has(metric.path)
              : activePaths.has(metric.path)
          ),
      );
      if (mentioned.length === 1) {
        return {
          name: action,
          arguments: { refs: [mentioned[0].ref] },
        };
      }
    }
    if (
      (
        action === "read_latest_metric" ||
        action === "select_metric_variant" ||
        action === "build_metric_chart" ||
        action === "add_chart_series" ||
        action === "remove_chart_series" ||
        action === "replace_chart_series"
      ) &&
      metrics.length === 1
    ) {
      return {
        name: action,
        arguments: { refs: [metrics[0].ref] },
      };
    }

    if (action === "list_metric_variants") {
      const variants = evidence.metricOptions.filter(
        ({ origin }) => origin === "variant",
      );
      if (variants.length === 1) {
        return {
          name: action,
          arguments: { refs: [variants[0].ref] },
        };
      }
      const bases = evidence.metricOptions.filter(
        ({ origin }) => origin === "mentioned" || origin === "context",
      );
      if (!variants.length && bases.length === 1) {
        return {
          name: action,
          arguments: { refs: [bases[0].ref] },
        };
      }
      return undefined;
    }

    if (action === "set_chart_view_scale") {
      const request = ` ${normalize(this.question)} `;
      const styles = actionTool(evidence, action).function.parameters
        .properties.styles.items.enum
        .filter((/** @type {string} */ style) =>
          request.includes(` ${normalize(style)} `)
        );
      if (styles.length) {
        return {
          name: action,
          arguments: { styles },
        };
      }
    }

    if (action !== "explain_evidence") return undefined;
    const { context, metricOptions, sourceOptions, guideOptions } = evidence;
    const contextual = context.metrics[0]
      ? metricOptions.find(({ metric }) =>
          metric.path === context.metrics[0].path
        )
      : undefined;
    const mentioned = metricOptions.find(({ origin }) => origin === "mentioned");
    const metric = mentioned ?? contextual;
    const grounding = sourceOptions[0] ?? guideOptions[0];
    if (!metric || !grounding) return undefined;
    return {
      name: action,
      arguments: {
        refs: [grounding.ref],
        metrics: [metric.ref],
      },
    };
  }

  /** @param {string} action */
  actionMessages(action) {
    const scopedEvidence = this.evidence;
    if (!scopedEvidence) throw new Error("Tool session is not ready");
    const { context, apiOptions, sourceOptions, guideOptions } =
      scopedEvidence;
    const actionMetrics = capabilityMetrics(scopedEvidence, action);
    /** @type {Record<string, unknown>} */
    const evidence = {
      request: this.question,
    };

    if (
      action === "set_chart_view_scale" ||
      action === "add_chart_series" ||
      action === "remove_chart_series" ||
      action === "replace_chart_series"
    ) {
      evidence.activeChart = context.chart?.chart;
    }
    if ([
      "read_latest_metric",
      "read_metric_at",
      "read_metric_range",
      "build_metric_chart",
      "add_chart_series",
      "remove_chart_series",
      "replace_chart_series",
      "list_metric_variants",
      "select_metric_variant",
      "explain_evidence",
    ].includes(action)) {
      evidence.metrics = actionMetrics.map(
        (/** @type {any} */ { ref, label, metric, origin }) => ({
          ref,
          label,
          origin,
          path: metric.path,
          indexes: action.startsWith("read_metric") ||
              action === "read_latest_metric"
            ? metric.indexes
            : undefined,
          unit: metric.suggestedUnit,
        }),
      );
    }
    if (action === "explain_evidence" || action === "search_source") {
      evidence.source = sourceOptions.map(
        (/** @type {any} */ { ref, source }) => ({
        ref,
        path: source.path,
        startLine: source.startLine,
        content: source.content.slice(0, 220),
        }),
      );
      evidence.guides = guideOptions.map(({ ref, guide }) => ({
        ref,
        title: guide.title,
        description: guide.description,
      }));
      if (context.source.length) {
        evidence.previousSource = context.source.map(
          (/** @type {any} */ source) => ({
            path: source.path,
            startLine: source.startLine,
            content: source.content,
          }),
        );
      }
      if (context.knowledge) {
        evidence.previousAnswer = context.knowledge.description;
      }
    }
    if (action === "call_api") {
      evidence.api = apiOptions.map(({ ref, operation }) => ({
        ref,
        operation: `${operation.method} ${operation.path}`,
        summary: operation.summary || operation.label,
        description: operation.description,
        parameters: operation.parameters,
        response: {
          type: operation.response.type,
          description: operation.response.description,
          fields: operation.response.fields
            .slice(0, 12)
            .map((/** @type {any} */ { name, type, description }) => ({
              name,
              type,
              description,
            })),
        },
      }));
      if (context.api) {
        evidence.previousApi = {
          operation: context.api.operation.label,
          arguments: context.api.arguments,
        };
      }
    }
    if (
      (action === "answer_general" || action === "find_chart_metrics") &&
      context.knowledge
    ) {
      evidence.context = context.knowledge;
    }
    return [
      {
        role: /** @type {const} */ ("system"),
        content: actionInstruction(action),
      },
      {
        role: /** @type {const} */ ("user"),
        content: JSON.stringify(evidence),
      },
    ];
  }

  /** @param {string} ref */
  apiArgumentTool(ref) {
    if (!this.refs) throw new Error("Tool session is not ready");
    return apiArgumentTool(this.refs.get(ref, "api"));
  }

  /** @param {string} ref */
  apiArgumentMessages(ref) {
    if (!this.refs) throw new Error("Tool session is not ready");
    const operation = this.refs.get(ref, "api");
    return [
      {
        role: /** @type {const} */ ("system"),
        content: "Call provide_api_arguments exactly once. Copy only parameter values explicitly supplied by the newest request or verified API context. Omit everything else.",
      },
      {
        role: /** @type {const} */ ("user"),
        content: JSON.stringify({
          request: this.question,
          operation: {
            method: operation.method,
            path: operation.path,
            parameters: operation.parameters,
          },
          ...(this.evidence?.context.api
            ? {
                previousArguments: this.evidence.context.api.arguments,
              }
            : {}),
        }),
      },
    ];
  }

  /** @param {string} ref */
  apiArguments(ref) {
    if (!this.refs) throw new Error("Tool session is not ready");
    const operation = this.refs.get(ref, "api");
    return {
      ...(reusableArguments(operation, this.evidence?.context.api) ?? {}),
      ...explicitArguments(operation, this.question),
    };
  }

  /** @param {string} ref @param {Record<string, unknown>} arguments_ */
  hasApiArguments(ref, arguments_) {
    if (!this.refs) throw new Error("Tool session is not ready");
    return hasRequiredArguments(this.refs.get(ref, "api"), arguments_);
  }

  /** @param {string} ref @param {Record<string, unknown>} arguments_ */
  validateApiArguments(ref, arguments_) {
    if (!this.refs) throw new Error("Tool session is not ready");
    return validatedArguments(
      this.refs.get(ref, "api"),
      arguments_,
      this.question,
      this.evidence?.context.api,
    );
  }

  /** @param {string} ref @param {Record<string, unknown>} arguments_ */
  missingApiArguments(ref, arguments_) {
    if (!this.refs) throw new Error("Tool session is not ready");
    return this.refs.get(ref, "api").parameters
      .filter((/** @type {any} */ parameter) =>
        parameter.required && !Object.hasOwn(arguments_, parameter.name)
      );
  }

  /** @param {{ name: string, arguments: Record<string, unknown> }} call @param {(status: string) => void} onStatus @param {AbortSignal} signal */
  execute(call, onStatus, signal) {
    if (!this.executor) throw new Error("Tool session is not ready");
    return this.executor.execute(call, onStatus, signal);
  }
}
