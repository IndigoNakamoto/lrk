import { normalize, tokenAffinity } from "../text.js";

const CHART_STYLES = ["line", "area", "stacked", "bar", "dots", "linear", "log"];
const CHART_STYLE_ALIASES = new Map([
  ["bars", "bar"],
  ["logarithm", "log"],
  ["logarithmic", "log"],
]);
const ACTION_VOCABULARY = new Map([
  [
    "search_source",
    [
      "called",
      "caller",
      "callers",
      "code",
      "implemented",
      "implementation",
      "source",
      "usage",
      "usages",
    ],
  ],
]);

/** @param {string} question */
export function requestedChartStyles(question) {
  const requested = new Set(normalize(question).split(" "));
  return [...new Set(
    [...requested]
      .map((term) =>
        CHART_STYLES.includes(term) ? term : CHART_STYLE_ALIASES.get(term)
      )
      .filter(Boolean),
  )];
}

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
      "list_metric_cohorts_variants",
    );
  }
  if (evidence.guideOptions.length || evidence.metricOptions.length) {
    actions.push("explain_metric_calculation");
  }
  if (
    evidence.guideOptions.some(
      (/** @type {any} */ { guide }) => guide.example,
    )
  ) {
    actions.push("show_guide_example");
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
  list_metric_cohorts_variants: "Choose only when the requested result is a list of available cohorts, groupings, or series variants.",
  select_metric_variant: "Choose when the request selects one matched cohort or series variant without requesting a value or chart yet.",
  explain_metric_calculation: "Choose for a Bitview metric definition grounded in matched metric evidence.",
  show_guide_example: "Choose when the requested result is an example supplied by a matched Learn guide.",
  find_chart_metrics: "Choose when the user asks which chart metrics exist but no exact metric matched yet.",
  search_source: "Choose when the requested output explains, locates, or finds usages of BRK repository code.",
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
export function directAction(evidence, question, allowContext = false) {
  const hasCurrentExplanationEvidence =
    evidence.metricOptions.some(
      (/** @type {any} */ { origin }) => origin === "mentioned",
    ) ||
    evidence.guideOptions.some(
      (/** @type {any} */ { origin }) => origin === "current",
    );
  const actions = availableActions(evidence).filter((action) =>
    action !== "answer_general" &&
    action !== "clarify" &&
    (
      action !== "explain_metric_calculation" ||
      hasCurrentExplanationEvidence
    )
  );
  if (
    evidence.context.chart &&
    requestedChartStyles(question).length
  ) {
    return "set_chart_view_scale";
  }
  const queryTerms = terms(question);
  const vocabularyMatches = actions.filter((action) =>
    (ACTION_VOCABULARY.get(action) ?? []).some((candidate) =>
      [...queryTerms].some((term) =>
        tokenAffinity(term, candidate) >= 0.68
      )
    )
  );
  if (vocabularyMatches.length === 1) return vocabularyMatches[0];
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
        tokenAffinity(queryTerm, actionTerm) >= 0.68
      ) {
        matched.add(actionsForTerm[0]);
      }
    }
  }
  if (matched.size === 1) {
    const action = [...matched][0];
    if (
      action === "show_guide_example" &&
      !allowContext &&
      !evidence.guideOptions.some(
        (/** @type {any} */ { guide }) =>
          guide.example && guide.origin === "current",
      )
    ) {
      return undefined;
    }
    return action;
  }

  const variants = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "variant",
  );
  const mentioned = evidence.metricOptions.filter(
    (/** @type {any} */ { origin }) => origin === "mentioned",
  );
  if (variants.length === 1 && matched.size > 1) {
    const resultActions = [...matched].filter((action) =>
      action !== "list_metric_cohorts_variants" &&
      action !== "select_metric_variant"
    );
    if (resultActions.length === 1) return resultActions[0];
  }
  if (
    matched.size === 0 &&
    variants.length + mentioned.length === 1 &&
    evidence.context.capability === "read_latest_metric"
  ) {
    return evidence.context.capability;
  }
  if (
    matched.size === 0 &&
    !evidence.context.knowledge &&
    evidence.guideOptions.length > 0 &&
    (
      evidence.guideOptions.length === 1 ||
      Number(evidence.guideOptions[0].guide.score ?? 0) -
          Number(evidence.guideOptions[1].guide.score ?? 0) >= 5
    ) &&
    mentioned.length === 0
  ) {
    return "explain_metric_calculation";
  }
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
    ? action === "build_metric_chart"
      ? [...new Map(
        [...mentioned, ...contextual].map((option) => [
          option.metric.path,
          option,
        ]),
      ).values()]
      : mentioned
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
        continuesContext: {
          type: "boolean",
          description: "True only when the newest request continues the single active conversational subject rather than introducing another subject.",
        },
      },
      ["capability", "continuesContext"],
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
            enum: CHART_STYLES,
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
  if (action === "list_metric_cohorts_variants") {
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
  if (action === "explain_metric_calculation") {
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
  if (action === "show_guide_example") {
    const examples = guideOptions.filter(
      (/** @type {any} */ { guide }) => guide.example,
    );
    return tool(
      action,
      "Select a matched Learn guide that supplies the requested example.",
      { refs: references(examples, 1) },
      ["refs"],
    );
  }
  if (action === "search_source") {
    return tool(
      action,
      "Inspect the current BRK source snapshot before answering.",
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
        topic: {
          type: "string",
          description: "A short standalone subject for future follow-ups. Preserve the verified previous topic when the newest request continues it; replace it when the request clearly introduces another subject.",
        },
        explicitSubject: {
          type: "string",
          description: "A noun phrase explicitly naming a subject in the newest request. Never copy a question, command, pronoun, or request phrase. Use an empty string when the request is indirect.",
        },
        quantityUse: {
          type: "string",
          enum: ["none", "verified", "hypothetical", "unsupported"],
          description: "Classify quantities in the answer: none has no quantities; verified copies only request or context quantities; hypothetical uses clearly illustrative quantities that do not claim actual data; unsupported claims an actual quantity absent from verified context.",
        },
      },
      ["answer", "topic", "explicitSubject", "quantityUse"],
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

export const ROUTE_INSTRUCTION = `Choose one capability for the newest request from verified context and matches. Catalog matches are retrieval hints, not user intent: never turn a broad topic or conversation request into a metric, API, chart, or source action unless the requested output asks for data, a value, a chart, a record, an endpoint, or code. Treat context.activeCapability as the active tool mode: continue it for an elliptical follow-up unless the newest request clearly selects a different available output. A generic action applied to an indirect reference continues the single active subject and must not use clarify.

Set continuesContext true only when the newest request continues the single active subject, including indirect and ordinal references. Set it false when the request introduces a different subject or no active subject exists.

Examples: with activeCapability source, "explain it" means search_source and continuesContext true. With activeCapability metric, "what is its latest value?" means read_latest_metric and true. With an active API list, "show the first one" means call_api and true. Without a matched quantitative subject, "give me some numbers" means clarify. A request naming a different subject sets continuesContext false.
The requested output wins: edit an active chart with its edit/style capability; otherwise choose the matching chart, latest-value, historical-value, range, variant-list, or variant-selection capability.
Use call_api when the request asks to show, read, inspect, or retrieve a concrete matched API resource or its contextual follow-up. Choose it even when a required identifier is missing; the endpoint schema will ask for that identifier. Use explain_metric_calculation for a metric definition, find_chart_metrics only when the request asks about a chart, metric, or series and none matched yet, describe_capabilities only for a request about the assistant itself, and answer_general for ordinary Bitcoin knowledge or conversation.
Use show_guide_example when the user asks for an example and a matched Learn guide supplies one.
When context.concept exists, follow-up requests for a reason, explanation, example, or comparison use answer_general unless the newest request explicitly asks for a chart, value, API record, or repository source.
Use search_source only when the requested output explains, locates, or finds usages of BRK repository code. Resolve indirect or ordinal source references from context.sourceSubjects. Never choose it merely because source matches exist.
Use clarify when essential information is missing. In particular, a requested quantitative result without a matched metric, API resource, or quantitative context needs one concise clarification instead of a qualitative answer or guessed dataset.
Call choose_capability exactly once.`;

/** @param {string} action */
export function actionInstruction(action) {
  const common = `Call ${action} exactly once. Copy selected refs and explicit values exactly. Never invent evidence, values, or arguments. Put only positively requested subjects in refs and explicitly rejected subjects in excludedRefs when that field is available; never put the same ref in both. Dependent follow-ups prefer context-origin subjects; newly named subjects prefer mentioned-origin matches. Similar candidates are alternatives, not a reason to select all of them.`;
  if (action === "answer_general") {
    return "Call answer_general exactly once. A vague request for numbers without an exact metric, resource, or timeframe must answer with one concise clarification question and quantityUse none; never draft sample observations. Otherwise directly answer the newest request in at most 30 words and never repeat or paraphrase the question as the answer. explicitSubject must be only a noun phrase explicitly naming a subject in the newest request: never copy a question, command, pronoun, or request phrase, and use an empty string for an indirect follow-up. Resolve indirect follow-ups from the preceding assistant answer. If the newest request names another subject, ignore previous context and answer that new Bitcoin subject with one high-level, well-established sentence and no implementation details. When the preceding answer is verified and the request continues it, use only its facts and direct logical consequences. Without verified context, do not invent implementation components or make claims about trust, centralization, security, failure modes, or loss of funds; say when the requested detail cannot be answered reliably. Every claim must apply specifically to Bitcoin, not generic blockchain systems or Bitview's product, datasets, or availability. Use hypothetical quantities only when the newest request explicitly asks for a hypothetical, example, or illustration. Never invent observations, live, current, or real-time values. Set topic to the short standalone subject actually answered, preserving the preceding topic for an indirect follow-up, and classify quantityUse accurately. Do not mention routing, candidates, schemas, or internal instructions.";
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
  if (action === "explain_metric_calculation") {
    return `${common} Select the one excerpt that directly answers the request. For a metric definition, prefer its computation or formula over UI configuration, imports, aggregation, or downstream usage. Select matching metrics when the request is about a metric.`;
  }
  if (action === "show_guide_example") {
    return `${common} Select the one matched guide whose canonical example answers the request.`;
  }
  if (action === "search_source") {
    return `${common} Produce one compact lexical code-search query, not an answer. Resolve ordinals and indirect references from previousContext.subjects, then preserve the exact relevant symbol.`;
  }
  if (action === "find_chart_metrics") {
    return `${common} Produce one compact catalog query containing only the metric subjects requested or referenced in verified context.`;
  }
  if (action === "call_api") {
    return `${common} Choose one directly matching operation. Copy identifiers exactly and never invent a required argument.`;
  }
  if (action === "clarify") {
    return `${common} Ask exactly one direct question for the essential missing value.`;
  }
  return common;
}
