import { normalize, tokenAffinity } from "../text.js";

/** @param {string} name @param {string} description @param {Record<string, any>} properties @param {string[]} required */
function tool(name, description, properties = {}, required = []) {
  return {
    type: "function",
    function: {
      name,
      description,
      parameters: {
        type: "object",
        properties,
        required,
        additionalProperties: false,
      },
    },
  };
}

/** @param {{ ref: string }[]} options @param {number} [maxItems] @param {string} [description] */
function references(options, maxItems = 4, description) {
  return {
    type: "array",
    minItems: 1,
    maxItems,
    ...(description ? { description } : {}),
    items: {
      type: "string",
      enum: options.map(({ ref }) => ref),
    },
  };
}

/** @param {import("../api/index.js").ApiOperation[]} operations */
function apiArguments(operations) {
  const parameters = new Map();
  for (const operation of operations) {
    for (const parameter of operation.parameters) {
      const current = parameters.get(parameter.name);
      parameters.set(parameter.name, {
        type: "string",
        description: [
          current?.description,
          `${parameter.in}${parameter.required ? ", required" : ""} for ${operation.path}${parameter.description ? `: ${parameter.description}` : ""}`,
        ].filter(Boolean).join(" "),
      });
    }
  }
  return Object.fromEntries(parameters);
}

/** @param {any} evidence */
export function availableActions(evidence) {
  const actions = [];
  const hasVariantSelection = evidence.context.metrics.length &&
    evidence.metricOptions.some(
      (/** @type {any} */ { origin }) => origin === "variant",
    );
  if (evidence.apiOptions.length) actions.push("call_api");
  if (evidence.context.chart) {
    const activePaths = new Set(
      evidence.context.chart.chart.series.map(
        (/** @type {{ path: string }} */ { path }) => path,
      ),
    );
    if (
      evidence.metricOptions.some(
        (/** @type {any} */ { metric }) => !activePaths.has(metric.path),
      )
    ) {
      actions.push("add_chart_series");
    }
    if (
      evidence.metricOptions.some(
        (/** @type {any} */ { metric }) => activePaths.has(metric.path),
      )
    ) {
      actions.push("remove_chart_series");
    }
    if (evidence.metricOptions.length) actions.push("replace_chart_series");
  }
  if (evidence.context.chart) actions.push("set_chart_view_scale");
  if (hasVariantSelection) actions.push("select_metric_variant");
  if (evidence.metricOptions.length) {
    actions.push(
      "read_latest_metric",
      "read_metric_at",
      "read_metric_range",
      "build_metric_chart",
      "list_metric_variants",
    );
  }
  if (evidence.guideOptions.length || evidence.metricOptions.length) {
    actions.push("explain_evidence");
  }
  if (!evidence.metricOptions.length) actions.push("find_chart_metrics");
  actions.push("search_source");
  actions.push("describe_capabilities", "answer_general", "clarify");
  return actions;
}

/** @type {Record<string, string>} */
const ROUTE_DESCRIPTIONS = {
  add_chart_series: "Choose when the request adds series to activeChart.",
  remove_chart_series: "Choose when the request removes series from activeChart.",
  replace_chart_series: "Choose when the request replaces activeChart's series.",
  set_chart_view_scale: "Choose when the request changes activeChart's view or scale.",
  read_latest_metric: "Choose when the requested result is the latest value of a metric.",
  read_metric_at: "Choose when the requested result is a metric value at a stated block height, date, or position.",
  read_metric_range: "Choose when the requested result is metric values across a stated range.",
  build_metric_chart: "Choose when the requested result is a new metric chart or graph.",
  list_metric_variants: "Choose only when the requested result is a list of available cohorts, groupings, or series variants.",
  select_metric_variant: "Choose when the request selects one matched cohort or series variant without requesting a value or chart yet.",
  explain_evidence: "Choose for a Bitview metric definition grounded in matched metric evidence.",
  find_chart_metrics: "Choose when the user asks which chart metrics exist but no exact metric matched yet.",
  search_source: "Choose for a question about BRK repository code, implementation, callers, or source structure.",
  call_api: "Choose for a concrete blockchain record or resource when a generated operation can accept the supplied or contextual identifier.",
  describe_capabilities: "Choose only when the user asks what this assistant can do.",
  answer_general: "Choose for ordinary Bitcoin knowledge, conversation, or writing.",
  clarify: "Choose only when missing information would materially change the result.",
};

/** @param {unknown} value */
function terms(value) {
  return new Set(
    normalize(value).split(" ").filter((term) => term.length >= 3),
  );
}

/**
 * Resolve only terminology encoded in the capability identifiers themselves.
 * Natural-language descriptions contain incidental words and remain model
 * context rather than routing rules.
 *
 * @param {any} evidence
 * @param {string} question
 */
export function directAction(evidence, question) {
  const actions = availableActions(evidence).filter(
    (action) => action !== "answer_general" && action !== "clarify",
  );
  const owners = new Map();
  for (const action of actions) {
    for (const term of terms(action)) {
      const values = owners.get(term) ?? [];
      values.push(action);
      owners.set(term, values);
    }
  }
  const matched = new Set();
  for (const queryTerm of terms(question)) {
    for (const [actionTerm, actionsForTerm] of owners) {
      if (
        actionsForTerm.length === 1 &&
        tokenAffinity(queryTerm, actionTerm) >= 0.75
      ) {
        matched.add(actionsForTerm[0]);
      }
    }
  }
  if (matched.size === 1) return [...matched][0];

  const variants = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "variant",
  );
  return matched.size === 0 && variants.length === 1
    ? "select_metric_variant"
    : undefined;
}

export function generalCapabilities() {
  return [
    "Explain Bitcoin concepts and Bitview metrics from verified evidence",
    "Read blockchain records and metric values",
    "Build and edit metric charts",
    "Search the current BRK source code",
  ];
}

/** @param {any} evidence @param {string} action */
export function capabilityMetrics(evidence, action) {
  const mentioned = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "mentioned",
  );
  const contextual = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "context",
  );
  const recent = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "recent",
  );
  const variants = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "variant",
  );
  const options = mentioned.length
    ? [...new Map(
      [...mentioned, ...contextual].map((option) => [
        option.metric.path,
        option,
      ]),
    ).values()]
    : variants.length
      ? action === "build_metric_chart" && contextual.length
        ? [...variants, ...contextual]
        : variants
      : action === "build_metric_chart" && contextual.length && recent.length
        ? [...contextual, ...recent]
      : contextual.length
        ? contextual
        : recent.length
          ? recent
        : evidence.metricOptions;
  const activePaths = new Set(
    evidence.context.chart?.chart.series.map(
      (/** @type {{ path: string }} */ { path }) => path,
    ) ?? [],
  );
  if (action === "add_chart_series") {
    return options.filter(
      (/** @type {any} */ { metric }) => !activePaths.has(metric.path),
    );
  }
  if (action === "remove_chart_series") {
    return options.filter(
      (/** @type {any} */ { metric }) => activePaths.has(metric.path),
    );
  }
  return options;
}

/** @param {any} evidence */
export function routeTools(evidence) {
  const actions = availableActions(evidence);
  return [
    tool(
      "choose_capability",
      actions.map((action) => `${action}: ${ROUTE_DESCRIPTIONS[action]}`).join(" "),
      {
        capability: {
          type: "string",
          enum: actions,
        },
        ...(evidence.apiOptions.length
          ? {
            apiRef: {
              type: "string",
              enum: evidence.apiOptions.map(
                (/** @type {any} */ { ref }) => ref,
              ),
              description: "When capability is call_api, the one matching generated operation.",
            },
          }
          : {}),
        sourceQuery: {
          type: "string",
          description: "Provide only when capability is search_source: one compact lexical code-search query using symbols and implementation terms from the request and verified context.",
        },
      },
      ["capability"],
    ),
  ];
}

/** @param {any} evidence @param {string} action */
export function actionTool(evidence, action) {
  const { metricOptions, apiOptions, sourceOptions, guideOptions } = evidence;

  if (
    action === "add_chart_series" ||
    action === "remove_chart_series" ||
    action === "replace_chart_series"
  ) {
    const options = capabilityMetrics(evidence, action);
    return tool(
      action,
      ROUTE_DESCRIPTIONS[action],
      { refs: references(options, 6) },
      ["refs"],
    );
  }
  if (action === "set_chart_view_scale") {
    return tool(
      action,
      "Change only the explicitly requested active-chart view or scale.",
      {
        styles: {
          type: "array",
          minItems: 1,
          maxItems: 2,
          items: {
            type: "string",
            enum: ["line", "area", "stacked", "bar", "dots", "linear", "log"],
          },
        },
      },
      ["styles"],
    );
  }
  if (
    action === "read_latest_metric" ||
    action === "read_metric_at" ||
    action === "read_metric_range"
  ) {
    const indexes = [...new Set(
      metricOptions.flatMap(
        (/** @type {any} */ { metric }) => metric.indexes ?? [],
      ),
    )];
    return tool(
      action,
      ROUTE_DESCRIPTIONS[action],
      {
        refs: references(
          capabilityMetrics(evidence, action),
          4,
          "Only the metrics actually requested; omit negated alternatives.",
        ),
        excludedRefs: references(
          capabilityMetrics(evidence, action),
          4,
          "Metrics explicitly negated or excluded by the request. Never repeat these in refs.",
        ),
        ...(action === "read_metric_at"
          ? {
            at: {
              type: "string",
              description: "Exact block height, date, or position copied from the request.",
            },
          }
          : {}),
        ...(action === "read_metric_range"
          ? {
            index: {
              type: "string",
              ...(indexes.length ? { enum: indexes } : {}),
            },
          }
          : {}),
        ...(action === "read_metric_range"
          ? {
            start: { type: "string" },
            end: { type: "string" },
            points: { type: "integer", minimum: 1, maximum: 120 },
          }
          : {}),
      },
      [
        "refs",
        ...(action === "read_metric_at" ? ["at"] : []),
      ],
    );
  }
  if (action === "build_metric_chart") {
    const options = capabilityMetrics(evidence, action);
    const asksContextDecision =
      options.some((/** @type {any} */ { origin }) => origin === "context") &&
      options.some((/** @type {any} */ { origin }) => origin === "mentioned");
    return tool(
      action,
      "Select only the metrics requested for the new chart.",
      {
        refs: references(
          options,
          6,
          "Only positively requested chart series; omit negated or excluded alternatives.",
        ),
        excludedRefs: references(
          options,
          6,
          "Chart series explicitly negated or excluded by the request. Never repeat these in refs.",
        ),
        ...(asksContextDecision
          ? {
              includeContext: {
                type: "boolean",
                description: "Resolve indirect references first. True when the complete requested series set includes the current metric together with newly named metrics; false when the new subject replaces it.",
              },
            }
          : {}),
      },
      ["refs", ...(asksContextDecision ? ["includeContext"] : [])],
    );
  }
  if (action === "list_metric_variants") {
    return tool(
      action,
      "Select the one metric whose source-derived variants were requested.",
      { refs: references(metricOptions, 1) },
      ["refs"],
    );
  }
  if (action === "select_metric_variant") {
    return tool(
      action,
      "Select the one matched source-derived metric variant requested.",
      { refs: references(capabilityMetrics(evidence, action), 1) },
      ["refs"],
    );
  }
  if (action === "explain_evidence") {
    const evidenceOptions = [...sourceOptions, ...guideOptions];
    return tool(
      action,
      "Select the smallest sufficient verified evidence for the answer.",
      {
        refs: references(evidenceOptions, 1),
        ...(metricOptions.length
          ? { metrics: references(metricOptions, 4) }
          : {}),
      },
      ["refs"],
    );
  }
  if (action === "search_source") {
    return tool(
      action,
      "Search the current BRK source snapshot before answering.",
      {
        query: {
          type: "string",
          description: "One compact lexical code-search query containing useful symbols, identifiers, or implementation terms from the request and verified source context.",
        },
      },
      ["query"],
    );
  }
  if (action === "find_chart_metrics") {
    return tool(
      action,
      "Search the generated chart metric catalog.",
      {
        query: {
          type: "string",
          description: "One compact metric search query using subjects from the request and verified conversation context.",
        },
      },
      ["query"],
    );
  }
  if (action === "call_api") {
    return tool(
      action,
      "Select the one generated read-only operation that directly answers the request.",
      {
        ref: {
          type: "string",
          enum: apiOptions.map((/** @type {any} */ { ref }) => ref),
        },
      },
      ["ref"],
    );
  }
  if (action === "answer_general") {
    return tool(
      action,
      "Answer without claiming live Bitview data or repository evidence.",
      {
        answer: {
          type: "string",
          description: "A clear concise answer to the exact request.",
        },
      },
      ["answer"],
    );
  }
  if (action === "describe_capabilities") {
    return tool(
      action,
      "Describe the assistant's generated capabilities.",
    );
  }
  if (action === "clarify") {
    return tool(
      action,
      "Ask one concise question because essential information is missing.",
      { question: { type: "string" } },
      ["question"],
    );
  }
  throw new Error(`Unsupported capability: ${action}`);
}

/** @param {import("../api/index.js").ApiOperation} operation */
export function apiArgumentTool(operation) {
  return tool(
    "provide_api_arguments",
    `Copy only arguments for ${operation.method} ${operation.path}. Omit values not supplied by the request or verified context.`,
    {
      arguments: {
        type: "object",
        properties: apiArguments([operation]),
        additionalProperties: false,
      },
    },
    ["arguments"],
  );
}

export const ROUTE_INSTRUCTION = `Choose one capability for the newest request from verified context and matches. Treat context.activeCapability as the active tool mode: continue it for an elliptical follow-up unless the newest request clearly selects a different available output.
The requested output wins: edit an active chart with its edit/style capability; otherwise choose the matching chart, latest-value, historical-value, range, variant-list, or variant-selection capability.
Use call_api for a concrete blockchain resource or its contextual follow-up, explain_evidence for a metric definition, find_chart_metrics to discover real chart series when none matched yet, describe_capabilities only for a request about the assistant itself, and answer_general for ordinary Bitcoin knowledge or conversation.
Use search_source only when the request explicitly asks about BRK repository code, source location, implementation, or callers. Never choose it merely because source matches exist.
Use clarify when essential information is missing. In particular, a requested quantitative result without a matched metric, API resource, or quantitative context needs one concise clarification instead of a qualitative answer or guessed dataset. With call_api select apiRef. With search_source provide sourceQuery.
Call choose_capability exactly once.`;

/** @param {string} action */
export function actionInstruction(action) {
  const common = `Call ${action} exactly once. Copy selected refs and explicit values exactly. Never invent evidence, values, or arguments. Put only positively requested subjects in refs and explicitly rejected subjects in excludedRefs when that field is available; never put the same ref in both. Dependent follow-ups prefer context-origin subjects; newly named subjects prefer mentioned-origin matches. Similar candidates are alternatives, not a reason to select all of them.`;
  if (action === "answer_general") {
    return "Answer the newest request naturally in at most 60 words. Resolve elliptical follow-ups from the provided context without repeating that context first. Every claim must apply specifically to Bitcoin, not generic blockchain systems or Bitview's product, datasets, or availability. A request for examples needs at least three distinct named things, not a restatement or a list of benefits. Never invent observations, quantities, live, current, or real-time values. Do not mention routing, candidates, schemas, or internal instructions.";
  }
  if (action === "read_metric_at") {
    return `${common} Copy the requested historical position exactly into at.`;
  }
  if (action === "build_metric_chart") {
    return `${common} Resolve indirect references against the current metric, then select the complete requested chart series set.`;
  }
  if (
    action === "add_chart_series" ||
    action === "remove_chart_series" ||
    action === "replace_chart_series"
  ) {
    return `${common} Preserve the active chart and apply exactly the requested series change.`;
  }
  if (action === "set_chart_view_scale") {
    return `${common} Apply only the explicitly requested view or scale.`;
  }
  if (action === "explain_evidence") {
    return `${common} Select the one excerpt that directly answers the request. For a metric definition, prefer its computation or formula over UI configuration, imports, aggregation, or downstream usage. Select matching metrics when the request is about a metric.`;
  }
  if (action === "search_source") {
    return `${common} Produce one compact lexical code-search query, not an answer. Preserve relevant symbols and identifiers from verified source context.`;
  }
  if (action === "find_chart_metrics") {
    return `${common} Produce one compact catalog query containing only the metric subjects requested or referenced in verified context.`;
  }
  if (action === "call_api") {
    return `${common} Choose one directly matching operation. Copy identifiers exactly and never invent a required argument.`;
  }
  return common;
}
