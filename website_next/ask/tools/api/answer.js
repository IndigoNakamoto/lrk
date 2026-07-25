import { renderApiAnswer } from "../render.js";
import { normalize } from "../text.js";
import { focusApiData } from "./result.js";

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

  return {
    fields,
    tool: {
      type: "function",
      function: {
        name: "answer_from_api",
        description: "Answer only from verified API data. Use calculate whenever the requested numeric result combines fields.",
        parameters: {
          type: "object",
          properties: {
            action: { type: "string", enum: ["calculate", "answer"] },
            label: {
              type: "string",
              description: "Short user-facing name for a calculated result.",
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
  return normalize(value).split(" ").filter(Boolean);
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
  /** @type {{ left: string, right: string, context: string, label: string } | undefined} */
  let expression;
  const minus = question.match(/^(.*?)(?:,\s*)?([^,;?.]+?)\s+minus\s+([^,;?.]+)[?.]*$/i);
  if (minus) {
    expression = {
      context: minus[1],
      left: minus[2],
      right: minus[3],
      label: `${minus[2].trim()} minus ${minus[3].trim()}`,
    };
  } else {
    const difference = question.match(
      /^(.*?)\bdifference\s+between\s+([^,;?.]+?)\s+and\s+([^,;?.]+)[?.]*$/i,
    );
    if (difference) {
      expression = {
        context: difference[1],
        left: difference[2],
        right: difference[3],
        label: `difference between ${difference[2].trim()} and ${difference[3].trim()}`,
      };
    } else {
      const subtract = question.match(
        /^(.*?)\bsubtract\s+([^,;?.]+?)\s+from\s+([^,;?.]+)[?.]*$/i,
      );
      if (subtract) {
        expression = {
          context: subtract[1],
          left: subtract[3],
          right: subtract[2],
          label: `${subtract[3].trim()} minus ${subtract[2].trim()}`,
        };
      }
    }
  }
  if (!expression) return undefined;

  /** @param {string} phrase */
  const select = (phrase) => {
    const ranked = fields
      .map((field) => ({
        field,
        score: fieldScore(phrase, expression.context, field),
      }))
      .filter(({ score }) => score > 0)
      .sort((left, right) => right.score - left.score);
    if (!ranked.length || ranked[1]?.score === ranked[0].score) return undefined;
    return ranked[0].field;
  };
  const left = select(expression.left);
  const right = select(expression.right);
  if (!left || !right || left.ref === right.ref) return undefined;

  return finishApiAnswer(
    {
      action: "calculate",
      label: expression.label,
      terms: [
        { ref: left.ref, sign: "add" },
        { ref: right.ref, sign: "subtract" },
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
  if (action.action !== "calculate" || !Array.isArray(action.terms) || !action.terms.length) {
    throw new Error("The AI returned an invalid API calculation");
  }
  const byRef = new Map(fields.map((field) => [field.ref, field]));
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
