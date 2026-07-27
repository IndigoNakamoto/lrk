import { renderApiAnswer } from "../render.js";
import { normalize } from "../text.js";
import { focusApiData } from "./result.js";
import { apiRequestWords } from "./routing.js";

const MAX_FIELDS = 64;

/**
 * @typedef {Object} ApiNumericField
 * @property {string} ref
 * @property {string} name
 * @property {string} type
 * @property {string} [description]
 * @property {number} value
 *
 * @typedef {Object} ApiAnswerSpec
 * @property {ApiNumericField[]} fields
 * @property {any} tool
 */

/** @param {unknown} value @param {string[]} path */
function valueAt(value, path) {
  let current = value;
  for (const key of path) {
    if (!current || typeof current !== "object" || !Object.hasOwn(current, key)) {
      return undefined;
    }
    current = /** @type {Record<string, unknown>} */ (current)[key];
  }
  return current;
}

/** @param {any} grounding @returns {ApiAnswerSpec} */
export function createApiAnswerTool(grounding) {
  const data = focusApiData(grounding.data, grounding.arguments);
  const responseFields = /** @type {{ name: string, type: string, description?: string }[]} */ (
    grounding.operation.response.fields ?? []
  );
  const fields = responseFields
    .map((field) => ({
      ...field,
      value: valueAt(data, field.name.split(".")),
    }))
    .filter((field) => typeof field.value === "number")
    .slice(0, MAX_FIELDS)
    .map((field, index) => ({
      ...field,
      value: /** @type {number} */ (field.value),
      ref: `n${index + 1}`,
    }));
  const fieldDescription = fields
    .map((field) =>
      `${field.ref}=${field.name} (${field.type}): ${field.value}${field.description ? ` — ${field.description}` : ""}`
    )
    .join("; ");
  const canCalculate = fields.length > 0;

  return {
    fields,
    tool: {
      type: "function",
      function: {
        name: "answer_from_api",
        description: canCalculate
          ? "Answer only from verified API data. Use calculate whenever the requested numeric result combines fields."
          : "Answer only from verified API data.",
        parameters: {
          type: "object",
          properties: {
            action: {
              type: "string",
              enum: canCalculate ? ["calculate", "answer"] : ["answer"],
            },
            label: {
              type: "string",
              description: "Short user-facing name for a calculated result.",
            },
            ...(canCalculate
              ? {
                operator: {
                  type: "string",
                  enum: ["add", "subtract", "multiply", "divide"],
                  description: "Arithmetic operator for operands. Use terms instead for a signed sum.",
                },
                operands: {
                  type: "array",
                  minItems: 2,
                  maxItems: 12,
                  items: {
                    type: "string",
                    enum: fields.map(({ ref }) => ref),
                    description: `Verified numeric fields: ${fieldDescription}`,
                  },
                  description: "Ordered source fields for operator arithmetic.",
                },
                terms: {
                  type: "array",
                  minItems: 1,
                  maxItems: 12,
                  items: {
                    type: "object",
                    properties: {
                      ref: {
                        type: "string",
                        enum: fields.map(({ ref }) => ref),
                        description: `Verified numeric fields: ${fieldDescription}`,
                      },
                      sign: { type: "string", enum: ["add", "subtract"] },
                    },
                    required: ["ref", "sign"],
                    additionalProperties: false,
                  },
                  description: "Exact arithmetic expression, one signed term per source field.",
                },
              }
              : {}),
            text: {
              type: "string",
              description: "For answer only: concise answer copied or summarized from verified data, with no invented values.",
            },
          },
          required: ["action"],
          additionalProperties: false,
        },
      },
    },
  };
}

/** @param {string} value */
function words(value) {
  return [...apiRequestWords(value)];
}

/**
 * @param {string} phrase
 * @param {string} context
 * @param {ApiNumericField} field
 */
function fieldScore(phrase, context, field) {
  const name = normalize(field.name);
  const description = normalize(field.description ?? "");
  const document = new Set(words(`${name} ${description}`));
  const phraseWords = words(phrase);
  if (!phraseWords.length || !phraseWords.every((word) => document.has(word))) return 0;

  let score = phraseWords.reduce(
    (sum, word) => sum + (new Set(words(name)).has(word) ? 8 : 3),
    0,
  );
  score += phraseWords.length * 10;
  const normalizedPhrase = normalize(phrase);
  if (name.includes(normalizedPhrase)) score += 12;
  if (description.includes(normalizedPhrase)) score += 5;
  for (const word of new Set(words(context))) {
    if (document.has(word)) score += name.includes(word) ? 2 : 1;
  }
  return score;
}

/**
 * Resolve only explicit two-operand subtraction from OpenAPI-derived numeric
 * fields. Ambiguous matches fall back to the model.
 *
 * @param {string} question
 * @param {ApiNumericField[]} fields
 * @param {any} grounding
 */
export function directApiCalculation(question, fields, grounding) {
  const normalizedQuestion = normalize(question);
  const virtualSizeRequested =
    /\bvsize\b/.test(normalizedQuestion) ||
    /\bvirtual size\b/.test(normalizedQuestion);
  const weight = fields.find((field) =>
    /\bweight\b/.test(normalize(`${field.name} ${field.type}`))
  );
  const explicitVsize = fields.find((field) =>
    /\b(?:vsize|virtual size)\b/.test(normalize(`${field.name} ${field.description ?? ""}`))
  );
  if (virtualSizeRequested && (explicitVsize || weight)) {
    const value = explicitVsize?.value ?? Math.ceil(/** @type {ApiNumericField} */ (weight).value / 4);
    return renderApiAnswer(
      `**virtual size**: ${new Intl.NumberFormat("en-US").format(value)} VSize`,
      grounding.operation,
    );
  }

  const totalNoun =
    normalizedQuestion.match(/\btotal\s+([a-z0-9]+)\b/)?.[1] ??
    normalizedQuestion.match(/\b([a-z0-9]+)\s+(?:in\s+)?total\b/)?.[1];
  if (totalNoun) {
    const selected = fields.filter((field) =>
      fieldScore(totalNoun, "total", field) > 0
    );
    if (
      selected.length > 1 &&
      new Set(selected.map(({ type }) => normalize(type))).size === 1
    ) {
      return finishApiAnswer(
        {
          action: "calculate",
          label: `total ${totalNoun}`,
          operator: "add",
          operands: selected.map(({ ref }) => ref),
        },
        fields,
        grounding,
      );
    }
  }

  if (/\bfee\s*rate\b/i.test(question) && /\b(?:imply|derive|calculate|per)\b/i.test(question)) {
    const fee = fields.filter((field) => /\bfee\b/.test(normalize(field.name)));
    if (fee.length === 1 && explicitVsize) {
      return finishApiAnswer(
        {
          action: "calculate",
          label: "fee rate",
          operator: "divide",
          operands: [fee[0].ref, explicitVsize.ref],
        },
        fields,
        grounding,
      );
    }
    if (fee.length === 1 && weight) {
      const vsize = Math.ceil(weight.value / 4);
      const rate = fee[0].value / vsize;
      return renderApiAnswer(
        `**fee rate**: ${new Intl.NumberFormat("en-US", { maximumFractionDigits: 8 }).format(rate)} ${fee[0].type}/VSize`,
        grounding.operation,
      );
    }
  }

  /** @type {{ left: string, right: string, context: string } | undefined} */
  let expression;
  const cleaned = question.replace(/[?.;]+$/g, "").trim();
  const minus = cleaned.match(/^(.*?)\s+minus\s+(.+)$/i);
  if (minus) {
    const comma = minus[1].lastIndexOf(",");
    const context = comma >= 0 ? minus[1].slice(0, comma) : "";
    const left = (comma >= 0 ? minus[1].slice(comma + 1) : minus[1]).trim();
    expression = {
      context,
      left,
      right: minus[2],
    };
  } else {
    const difference = cleaned.match(
      /^(.*?)\bdifference\s+between\s+(.+?)\s+and\s+(.+)$/i,
    );
    if (difference) {
      expression = {
        context: difference[1],
        left: difference[2],
        right: difference[3],
      };
    } else {
      const subtract = cleaned.match(
        /^(.*?)\bsubtract\s+(.+?)\s+from\s+(.+)$/i,
      );
      if (subtract) {
        expression = {
          context: subtract[1],
          left: subtract[3],
          right: subtract[2],
        };
      }
    }
  }
  if (!expression) return undefined;

  /** @param {string} value */
  const phraseVariants = (value) => {
    const values = words(value);
    const phrases = [];
    for (let length = 1; length <= Math.min(values.length, 6); length += 1) {
      for (let start = 0; start + length <= values.length; start += 1) {
        phrases.push(values.slice(start, start + length).join(" "));
      }
    }
    return phrases;
  };

  /** @param {string} phrase @param {string} context */
  const rank = (phrase, context) =>
    fields
      .map((field) => phraseVariants(phrase)
        .map((variant) => ({
          field,
          phrase: variant,
          score: fieldScore(variant, context, field),
        }))
        .sort((left, right) => right.score - left.score)[0])
      .filter(({ score }) => score > 0)
      .sort((left, right) => right.score - left.score);

  const leftCandidates = rank(
    expression.left,
    `${expression.context} ${expression.right}`,
  );
  const rightCandidates = rank(
    expression.right,
    `${expression.context} ${expression.left}`,
  );
  /** @param {ApiNumericField} field */
  const parent = (field) => field.name.split(".").slice(0, -1).join(".");
  const leaf = (/** @type {ApiNumericField} */ field) => field.name.split(".").at(-1);
  if (!/\b(?:confirmed|mempool|pending|unconfirmed)\b/i.test(question)) {
    const leftLeaf = leftCandidates[0] ? leaf(leftCandidates[0].field) : undefined;
    const rightLeaf = rightCandidates[0] ? leaf(rightCandidates[0].field) : undefined;
    const leftGroup = leftCandidates.filter(({ field }) => leaf(field) === leftLeaf);
    const rightGroup = rightCandidates.filter(({ field }) => leaf(field) === rightLeaf);
    if (
      leftGroup.length &&
      rightGroup.length &&
      (leftGroup.length > 1 || rightGroup.length > 1) &&
      new Set(
        [...leftGroup, ...rightGroup].map(({ field }) => normalize(field.type)),
      ).size === 1
    ) {
      return finishApiAnswer(
        {
          action: "calculate",
          label: `${leftGroup[0].phrase} minus ${rightGroup[0].phrase}`,
          terms: [
            ...leftGroup.map(({ field }) => ({ ref: field.ref, sign: "add" })),
            ...rightGroup.map(({ field }) => ({ ref: field.ref, sign: "subtract" })),
          ],
        },
        fields,
        grounding,
      );
    }
  }
  const pairs = leftCandidates.flatMap((left) =>
    rightCandidates
      .filter((right) =>
        left.field.ref !== right.field.ref &&
        normalize(left.field.type) === normalize(right.field.type)
      )
      .map((right) => ({
        left,
        right,
        score: left.score + right.score +
          (parent(left.field) === parent(right.field) ? 5 : 0),
      }))
  ).sort((left, right) => right.score - left.score);
  const [pair, second] = pairs;
  if (!pair || second?.score === pair.score) return undefined;

  return finishApiAnswer(
    {
      action: "calculate",
      label: `${pair.left.phrase} minus ${pair.right.phrase}`,
      terms: [
        { ref: pair.left.field.ref, sign: "add" },
        { ref: pair.right.field.ref, sign: "subtract" },
      ],
    },
    fields,
    grounding,
  );
}

/** @param {Record<string, unknown>} action @param {ApiNumericField[]} fields @param {any} grounding */
export function finishApiAnswer(action, fields, grounding) {
  if (action.action === "answer") {
    const text = typeof action.text === "string" ? action.text.trim() : "";
    if (!text) throw new Error("The AI returned an empty API answer");
    return renderApiAnswer(text, grounding.operation);
  }
  if (action.action !== "calculate") {
    throw new Error("The AI returned an invalid API calculation");
  }
  const byRef = new Map(fields.map((field) => [field.ref, field]));
  if (
    typeof action.operator === "string" &&
    ["add", "subtract", "multiply", "divide"].includes(action.operator) &&
    Array.isArray(action.operands) &&
    action.operands.length >= 2
  ) {
    const selected = action.operands.map((ref) => byRef.get(String(ref)));
    if (selected.some((field) => !field)) throw new Error("Unknown calculation field");
    const numeric = /** @type {ApiNumericField[]} */ (selected);
    const [first, ...rest] = numeric;
    const value = rest.reduce((result, field) => {
      if (action.operator === "add") return result + field.value;
      if (action.operator === "subtract") return result - field.value;
      if (action.operator === "multiply") return result * field.value;
      if (field.value === 0) throw new Error("Cannot divide by zero");
      return result / field.value;
    }, first.value);
    const unit = action.operator === "divide" && numeric.length === 2
      ? ` ${first.type}/${numeric[1].type}`
      : new Set(numeric.map(({ type }) => type)).size === 1
        ? ` ${first.type}`
        : "";
    const label = typeof action.label === "string" && action.label.trim()
      ? action.label.trim()
      : "result";
    const formatted = new Intl.NumberFormat("en-US", { maximumFractionDigits: 8 }).format(value);
    return renderApiAnswer(`**${label}**: ${formatted}${unit}`, grounding.operation);
  }
  if (!Array.isArray(action.terms) || !action.terms.length) {
    throw new Error("The AI returned an invalid API calculation");
  }
  const selected = action.terms.map((raw) => {
    if (!raw || typeof raw !== "object") throw new Error("Invalid calculation term");
    const term = /** @type {Record<string, unknown>} */ (raw);
    const field = byRef.get(String(term.ref));
    if (!field) throw new Error("Unknown calculation field");
    if (term.sign !== "add" && term.sign !== "subtract") {
      throw new Error("Invalid calculation sign");
    }
    return { field, sign: term.sign };
  });
  const value = selected.reduce(
    (sum, { field, sign }) => sum + (sign === "add" ? field.value : -field.value),
    0,
  );
  const types = new Set(selected.map(({ field }) => field.type));
  const unit = types.size === 1 ? ` ${selected[0].field.type}` : "";
  const label = typeof action.label === "string" && action.label.trim()
    ? action.label.trim()
    : "result";
  const formatted = new Intl.NumberFormat("en-US", { maximumFractionDigits: 8 }).format(value);
  return renderApiAnswer(`**${label}**: ${formatted}${unit}`, grounding.operation);
}
