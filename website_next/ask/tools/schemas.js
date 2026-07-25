/** @param {Record<string, unknown>} properties @param {string[]} required */
function actionTool(properties, required) {
  return {
    type: "function",
    function: {
      name: "next_action",
      description: "Choose exactly one allowed next step.",
      parameters: {
        type: "object",
        properties,
        required,
        additionalProperties: false,
      },
    },
  };
}

/** @param {boolean} hasActiveChart @param {boolean} hasPrevious */
export function searchTool(hasActiveChart = false, hasPrevious = false) {
  return actionTool({
    action: { type: "string", enum: ["search"] },
    context: {
      type: "string",
      enum: hasPrevious
        ? ["new_topic", "reuse_previous", "extend_previous"]
        : ["new_topic"],
      description: "Reuse the previous verified topic for dependent follow-ups. Extend it only when the user adds another distinct metric. Otherwise start a new topic.",
    },
    queries: {
      type: "array",
      minItems: 1,
      maxItems: 4,
      items: { type: "string" },
      description: "One terse catalog-style Bitcoin metric or technical noun phrase per distinct topic. Translate the user's meaning. Never copy a question or include request verbs, pronouns, time words, or punctuation. For X vs Y, return separate complete X and Y metric phrases.",
    },
    cardinality: {
      type: "string",
      enum: ["single", "multiple"],
      description: "Use multiple whenever the user requests a comparison or more than one distinct metric, even if queries accidentally contains one item.",
    },
    outcome: {
      type: "string",
      enum: [
        "read_requested_value",
        "read_api",
        "build_requested_chart",
        ...(hasActiveChart ? ["edit_existing_chart"] : []),
        "explain_from_verified_facts",
        "answer_general",
        "clarify_request",
      ],
    },
    clarification: {
      type: "string",
      description: "Only for clarify_request: one short question that distinguishes the materially different interpretations.",
    },
  }, ["action", "outcome"]);
}

/** @param {string[]} queries */
export function rewriteTool(queries) {
  return actionTool({
    action: { type: "string", enum: ["rewrite"] },
    queries: {
      type: "array",
      minItems: queries.length,
      maxItems: queries.length,
      items: { type: "string" },
      description: `Rewrite each input independently, in the same order, without merging them: ${queries.map((query, index) => `${index + 1}=${query}`).join("; ")}.`,
    },
  }, ["action", "queries"]);
}

/** @param {{ ref: string, label: string, operation: import("./api/index.js").ApiOperation }[]} options */
export function apiResolveTool(options) {
  const parameters = new Map();
  for (const { operation } of options) {
    for (const parameter of operation.parameters) {
      const current = parameters.get(parameter.name);
      const descriptions = [
        current?.description,
        `${parameter.in}${parameter.required ? ", required" : ""} for ${operation.path}${parameter.description ? `: ${parameter.description}` : ""}`,
      ].filter(Boolean);
      parameters.set(parameter.name, {
        type: "string",
        description: [...new Set(descriptions)].join(" "),
      });
    }
  }
  return actionTool({
    action: { type: "string", enum: ["call_api", "clarify"] },
    ref: {
      type: "string",
      enum: options.map(({ ref }) => ref),
      description: `Read-only operations: ${options.map(({ ref, label, operation }) => {
        const params = operation.parameters
          .map((parameter) => `${parameter.name}${parameter.required ? "*" : ""}`)
          .join(", ");
        return `${ref}=${label} [${params || "no parameters"}]`;
      }).join("; ")}`,
    },
    arguments: {
      type: "object",
      properties: Object.fromEntries(parameters),
      additionalProperties: false,
      description: "Arguments copied from the user's request. Include every required parameter for the selected operation.",
    },
    text: { type: "string", description: "For clarify only: one short question." },
  }, ["action"]);
}

/**
 * @param {{ ref: string, label: string }[]} options
 * @param {string} outcome
 * @param {number} [maxItems]
 */
export function resolveTool(options, outcome, maxItems = 3) {
  const refs = {
    type: "array",
    minItems: 1,
    maxItems,
    items: { type: "string", enum: options.map(({ ref }) => ref) },
    description: `Available references: ${options.map(({ ref, label }) => `${ref}=${label}`).join("; ")}`,
  };

  if (outcome === "explain_from_verified_facts") {
    return actionTool({
      action: { type: "string", enum: ["answer", "clarify"] },
      refs,
      text: { type: "string", description: "For clarify only: one short question." },
    }, ["action"]);
  }

  if (outcome === "read_requested_value") {
    return actionTool({
      action: { type: "string", enum: ["read_data", "clarify"] },
      refs,
      mode: { type: "string", enum: ["latest", "at", "range"] },
      index: { type: "string", description: "Index such as height or day1." },
      at: { type: "string", description: "Block height or date for at mode." },
      start: { type: "string" },
      end: { type: "string" },
      points: { type: "integer", minimum: 1, maximum: 120 },
      text: { type: "string", description: "For clarify only: one short question." },
    }, ["action"]);
  }

  const editing = outcome === "edit_existing_chart";
  return actionTool({
    action: {
      type: "string",
      enum: [editing ? "edit_chart" : "build_chart", "clarify"],
    },
    refs,
    title: { type: "string" },
    operation: {
      type: "string",
      enum: ["add", "remove", "replace"],
      description: "For edit_chart, make exactly the requested change.",
    },
    text: { type: "string", description: "For clarify only: one short question." },
  }, ["action"]);
}

export function clarifyTool() {
  return actionTool({
    action: { type: "string", enum: ["clarify"] },
    text: { type: "string", description: "One necessary clarification question." },
  }, ["action", "text"]);
}
