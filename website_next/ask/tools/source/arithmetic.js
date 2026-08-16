import { normalize } from "../text.js";

const COMPOUND_ASSIGNMENT = /^(.+?)\s*(\+=|-=|\*=|\/=)\s*(.+?);?\s*$/;
const LOCAL_ASSIGNMENT =
  /^\s*(?:let|const|var)\s+([A-Za-z_]\w*)(?:\s*:[^=]+)?\s*=\s*(.+?);?\s*$/;
const COMMENT_FORMULA =
  /^\s*(?:\/\/\/?|#|\*)?\s*([A-Za-z_]\w*)\s*=\s*(.+?)\s*$/;

/** @param {string} value */
function metricTokens(value) {
  return normalize(value).split(" ").filter((token) => token.length > 2);
}

/** @param {string} line @param {string[]} tokens */
function overlap(line, tokens) {
  const words = new Set(normalize(line).split(" "));
  return tokens.filter((token) => words.has(token)).length;
}

/** @param {string} value */
function cleanExpression(value) {
  let cleaned = value
    .replace(/\.as_u\d+\(\)/g, "")
    .replace(/\bself\./g, "")
    .replace(/\b([A-Za-z_]\w*)_u\d+\b/g, "$1")
    .replace(/\s*\*\s*/g, " × ")
    .replace(/\s*\/\s*/g, " ÷ ")
    .replace(/\s+/g, " ")
    .replace(/;$/, "")
    .trim();
  while (/\(\(([^()]+)\)\)/.test(cleaned)) {
    cleaned = cleaned.replace(/\(\(([^()]+)\)\)/g, "($1)");
  }
  return cleaned;
}

/** @param {string} expression @param {string[]} preceding */
function expandLocals(expression, preceding) {
  let expanded = expression;
  for (let pass = 0; pass < 2; pass += 1) {
    const identifiers = new Set(expanded.match(/\b[A-Za-z_]\w*\b/g) ?? []);
    let changed = false;
    for (const line of [...preceding].reverse()) {
      const match = line.match(LOCAL_ASSIGNMENT);
      if (
        !match ||
        !identifiers.has(match[1]) ||
        !/(?:\s[+\-*/]\s)/.test(match[2])
      ) {
        continue;
      }
      expanded = expanded.replace(
        new RegExp(`\\b${match[1]}\\b`, "g"),
        `(${match[2]})`,
      );
      changed = true;
    }
    if (!changed) break;
  }
  return cleanExpression(expanded);
}

/** @param {string | undefined} unit */
function displayUnit(unit) {
  if (!unit) return "";
  return unit.length <= 5 ? unit.toUpperCase() : unit;
}

/**
 * Turn a literal source formula into a concise answer without asking the model
 * to invent meanings for code identifiers.
 *
 * @param {{ metrics: { name: string, unit?: string }[], excerpts: { content: string }[] }} grounding
 */
export function arithmeticAnswer(grounding) {
  if (grounding.metrics.length !== 1) return undefined;
  const metric = grounding.metrics[0];
  const tokens = metricTokens(metric.name);
  if (!tokens.length) return undefined;

  for (const { content } of grounding.excerpts) {
    const lines = content.split("\n");
    const candidates = lines
      .flatMap((line, index) => {
        const match = line.match(COMPOUND_ASSIGNMENT);
        return match
          ? [{ index, match, matched: overlap(match[1], tokens) }]
          : [];
      })
      .sort((left, right) =>
        right.matched - left.matched || left.index - right.index
      );
    const candidate = candidates[0];
    if (candidate?.matched) {
      const [, , operator, right] = candidate.match;
      const expression = expandLocals(
        right,
        lines.slice(0, candidate.index),
      );
      const action = operator === "+="
        ? "adds"
        : operator === "-="
          ? "subtracts"
          : operator === "*="
            ? "multiplies its running value by"
            : "divides its running value by";
      const target = operator === "+=" || operator === "-="
        ? `${action} \`${expression}\` to its running total`
        : `${action} \`${expression}\``;
      const unit = displayUnit(metric.unit);
      return `**${metric.name}** ${target}.${unit ? ` It is reported in ${unit}.` : ""}`;
    }

    const formulas = lines
      .flatMap((line, index) => {
        const match = line.match(COMMENT_FORMULA);
        const arithmetic = match &&
          /(?:\s[+\-*/]\s|[Σ∑])/.test(match[2]);
        return arithmetic
          ? [{ index, match, matched: overlap(match[1], tokens) }]
          : [];
      })
      .sort((left, right) =>
        right.matched - left.matched || left.index - right.index
      );
    const formula = formulas[0];
    if (formula?.matched) {
      const expression = cleanExpression(formula.match[2]);
      const unit = displayUnit(metric.unit);
      return `**${metric.name}** is calculated as \`${expression}\`.${unit ? ` It is reported in ${unit}.` : ""}`;
    }
  }
  return undefined;
}
