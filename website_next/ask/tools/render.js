import { formatValue } from "./data.js";
import { focusApiData } from "./api/result.js";
import { normalize } from "./text.js";

/**
 * @typedef {Object} MetricRead
 * @property {string} label
 * @property {string | undefined} unit
 * @property {string} index
 * @property {number | string} start
 * @property {string | undefined} stamp
 * @property {unknown[]} values
 */

/**
 * @typedef {Object} SourceEvidence
 * @property {string} revision
 * @property {string} path
 * @property {number} startLine
 * @property {number} [endLine]
 */

const SOURCE_URL = "https://github.com/bitcoinresearchkit/brk/blob";

/** @param {SourceEvidence} source */
function sourceKey(source) {
  return `${source.revision}:${source.path}:${source.startLine}:${source.endLine ?? source.startLine}`;
}

/** @param {SourceEvidence} source */
function sourceLink(source) {
  const end = source.endLine && source.endLine !== source.startLine
    ? `-${source.endLine}`
    : "";
  const path = source.path.split("/").map(encodeURIComponent).join("/");
  const lines = `#L${source.startLine}${end ? `-L${source.endLine}` : ""}`;
  const url = `${SOURCE_URL}/${encodeURIComponent(source.revision)}/${path}${lines}`;
  return `[\`${source.path}:${source.startLine}${end}\`](${url})`;
}

/** @param {{ facts: string[], sources: SourceEvidence[], excerpts: (SourceEvidence & { content: string })[] }} evidence */
export function renderEvidence(evidence) {
  const sections = [...new Set(evidence.facts)].filter(Boolean);
  const cited = new Set();
  for (const excerpt of evidence.excerpts) {
    sections.push(`${sourceLink(excerpt)}\n\n\`\`\`\n${excerpt.content}\n\`\`\``);
    cited.add(sourceKey(excerpt));
  }
  const sources = [...new Map(
    evidence.sources.map((source) => [sourceKey(source), source]),
  ).values()].filter((source) => !cited.has(sourceKey(source)));
  if (sources.length) {
    sections.push(`Source${sources.length === 1 ? "" : "s"}: ${sources.map(sourceLink).join(", ")}`);
  }
  return sections.join("\n\n") || "I could not find enough verified evidence to answer that.";
}

/** @param {MetricRead[]} results */
export function renderData(results) {
  return results.map((result) => {
    if (result.values.length === 1 && typeof result.values[0] === "number") {
      const position = result.index === "height"
        ? ` at block ${result.start}`
        : result.stamp
          ? ` at ${result.stamp}`
          : "";
      return `**${result.label}**: ${formatValue(result.values[0], result.unit)}${position}`;
    }
    const values = /** @type {number[]} */ (
      result.values.filter((value) => typeof value === "number")
    );
    if (!values.length) return `**${result.label}**: no values returned.`;
    return `**${result.label}**: ${values.length} values; latest ${formatValue(values[values.length - 1], result.unit)}.`;
  }).join("\n");
}

/** @param {string} answer @param {{ method: string, path: string }} operation */
export function renderApiAnswer(answer, operation) {
  return `${answer.trim()}\n\nData: \`${operation.method} ${operation.path}\``;
}

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

/** @param {unknown} value */
function scalar(value) {
  return value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean";
}

/** @param {unknown} value */
function displayScalar(value) {
  if (typeof value === "number") {
    return new Intl.NumberFormat("en-US", { maximumFractionDigits: 8 }).format(value);
  }
  if (typeof value === "string") return value;
  return JSON.stringify(value);
}

/** @param {{ field: { name: string, type: string }, value: unknown }} candidate */
function renderApiField(candidate) {
  const genericTypes = new Set([
    "boolean",
    "integer",
    "null",
    "number",
    "object",
    "string",
    "value",
  ]);
  const types = candidate.field.type
    .split("|")
    .map((type) => type.trim());
  const semanticTypes = types.filter((type) => !genericTypes.has(type.toLowerCase()));
  const unit = semanticTypes.length === 1 ? ` ${semanticTypes[0]}` : "";
  const label = candidate.field.name.replaceAll("_", " ").replaceAll(".", " · ");
  return `**${label}**: ${displayScalar(candidate.value)}${unit}`;
}

/**
 * Render scalar fields selected directly by exact OpenAPI field-name overlap.
 * Equal-scoring fields are returned together rather than asking the model to
 * choose, which is both faster and safer for questions such as "which block?".
 * Field names, parameter names, and units all come from OpenAPI.
 * @param {{ question: string, data: unknown, arguments?: Record<string, unknown>, operation: { method: string, path: string, parameters?: { name: string }[], response: { type?: string, fields?: { name: string, type: string, description?: string }[] } } }} grounding
 */
export function renderDirectApiAnswer(grounding) {
  if (
    /\b(?:add(?:ed)?|combined?|difference|minus|net|plus|subtract(?:ed)?|sum)\b/i
      .test(grounding.question)
  ) return undefined;

  const data = focusApiData(grounding.data, grounding.arguments);
  const responseFields = grounding.operation.response.fields ?? [];
  if (scalar(data) && !responseFields.length) {
    const type = grounding.operation.response.type ?? "value";
    const label = normalize(type) || "value";
    return renderApiAnswer(`**${label}**: ${displayScalar(data)}`, grounding.operation);
  }

  const words = new Set(
    normalize(grounding.question).match(/[a-z0-9]+/g) ?? [],
  );
  const parameters = new Set(
    (grounding.operation.parameters ?? []).map(({ name }) => normalize(name)),
  );
  const fields = responseFields.map((field) => {
    const nameTokens = new Set(normalize(field.name).match(/[a-z0-9]+/g) ?? []);
    const tokens = new Set(
      normalize(`${field.name} ${field.description ?? ""}`).match(/[a-z0-9]+/g) ?? [],
    );
    return { field, nameTokens, tokens };
  });
  const frequencies = new Map();
  for (const { tokens } of fields) {
    for (const token of tokens) {
      frequencies.set(token, (frequencies.get(token) ?? 0) + 1);
    }
  }
  const candidates = fields
    .map((field) => {
      const path = field.field.name.split(".");
      const leaf = path.at(-1) ?? "";
      let matches = 0;
      let score = 0;
      for (const word of words) {
        if (!field.tokens.has(word)) continue;
        matches += 1;
        const frequency = frequencies.get(word) ?? fields.length;
        const idf = Math.log((fields.length + 1) / (frequency + 1)) + 1;
        score += idf * (field.nameTokens.has(word) ? 3 : 1);
      }
      return {
        field: field.field,
        path,
        value: valueAt(data, path),
        matches,
        score,
        parameter: parameters.has(normalize(leaf)),
      };
    })
    .filter(({ matches, parameter, value }) =>
      matches > 0 && !parameter && scalar(value) && value !== null
    )
    .sort((left, right) => right.score - left.score || right.matches - left.matches);
  if (!candidates.length) return undefined;

  const selected = candidates
    .filter(({ score }) => score === candidates[0].score)
    .slice(0, 6);
  const answer = selected.length === 1
    ? renderApiField(selected[0])
    : selected.map((candidate) => `- ${renderApiField(candidate)}`).join("\n");
  return renderApiAnswer(answer, grounding.operation);
}
