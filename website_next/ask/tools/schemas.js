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

/** @param {boolean} hasActiveChart @param {boolean} hasPrevious @param {{ path: string, label: string }[]} [categories] */
export function searchTool(hasActiveChart = false, hasPrevious = false, categories = []) {
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
    families: {
      type: "array",
      minItems: 1,
      maxItems: 3,
      items: { type: "string", enum: ["auto", ...categories.map(({ path }) => path)] },
      description: `Use auto for exact terminology. Otherwise choose the narrowest source-derived family that expresses the user's meaning: ${categories.map(({ path, label }) => `${path}=${label}`).join("; ") || "auto only"}.`,
    },
    outcome: {
      type: "string",
      enum: [
        "read_requested_value",
        "build_requested_chart",
        ...(hasActiveChart ? ["edit_existing_chart"] : []),
        "explain_from_verified_facts",
      ],
    },
    focus: {
      type: "string",
      enum: ["definition", "variants", "implementation", "data"],
      description: "Choose variants for cohorts or availability, implementation for source code or calculation details, definition for conceptual explanations, and data for values or charts.",
    },
  }, ["action", "context", "queries", "cardinality", "families", "outcome", "focus"]);
}

/** @param {{ ref: string, label: string }[]} options */
export function navigateTool(options) {
  return actionTool({
    action: { type: "string", enum: ["navigate"] },
    refs: {
      type: "array",
      minItems: 1,
      maxItems: 3,
      items: { type: "string", enum: options.map(({ ref }) => ref) },
      description: `Metric families: ${options.map(({ ref, label }) => `${ref}=${label}`).join("; ")}`,
    },
  }, ["action", "refs"]);
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
      description: `Rewrite each input independently, in the same order, without merging them: ${queries.map((query, index) => `${index + 1}=${query}`).join("; ")}. For what one bitcoin is worth or trades for, use bitcoin spot price. Never use cointime unless explicitly requested.`,
    },
  }, ["action", "queries"]);
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
      action: { type: "string", enum: ["answer"] },
      refs,
    }, ["action", "refs"]);
  }

  if (outcome === "read_requested_value") {
    return actionTool({
      action: { type: "string", enum: ["read_data"] },
      refs,
      mode: { type: "string", enum: ["latest", "at", "range"] },
      index: { type: "string", description: "Index such as height or day1." },
      at: { type: "string", description: "Block height or date for at mode." },
      start: { type: "string" },
      end: { type: "string" },
      points: { type: "integer", minimum: 1, maximum: 120 },
    }, ["action", "refs", "mode"]);
  }

  const editing = outcome === "edit_existing_chart";
  return actionTool({
    action: { type: "string", enum: [editing ? "edit_chart" : "build_chart"] },
    refs,
    title: { type: "string" },
    operation: {
      type: "string",
      enum: ["add", "remove", "replace"],
      description: "For edit_chart, make exactly the requested change.",
    },
  }, ["action", "refs"]);
}

export function clarifyTool() {
  return actionTool({
    action: { type: "string", enum: ["clarify"] },
    text: { type: "string", description: "One necessary clarification question." },
  }, ["action", "text"]);
}
