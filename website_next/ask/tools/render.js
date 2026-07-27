import { formatValue } from "./data.js";
import { focusApiData } from "./api/result.js";
import { apiRequestWords } from "./api/routing.js";
import { normalize, tokenAffinity } from "./text.js";

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
const MIN_FIELD_AFFINITY = 0.65;

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

/** @param {unknown} data */
function apiRecords(data) {
  if (Array.isArray(data)) return { count: data.length, items: data };
  if (
    data &&
    typeof data === "object" &&
    typeof /** @type {{ count?: unknown }} */ (data).count === "number" &&
    Array.isArray(/** @type {{ sample?: unknown }} */ (data).sample)
  ) {
    return {
      count: /** @type {{ count: number }} */ (data).count,
      items: /** @type {{ sample: unknown[] }} */ (data).sample,
    };
  }
  return { count: undefined, items: [data] };
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
  const label = candidate.field.name.split(".").map(normalize).join(" · ");
  return `**${label}**: ${displayScalar(candidate.value)}${unit}`;
}

/**
 * Render a compact schema-derived sample when the user requests a resource
 * generally rather than one specific response field.
 *
 * @param {{ data: unknown, arguments?: Record<string, unknown>, operation: { method: string, path: string, summary?: string, response: { fields?: { name: string, type: string }[] } } }} grounding
 */
function renderApiOverview(grounding) {
  const data = focusApiData(grounding.data, grounding.arguments);
  const { count, items } = apiRecords(data);
  const fields = grounding.operation.response.fields ?? [];
  const samples = items.slice(0, 3).map((item) =>
    fields
      .map((field) => ({ field, value: valueAt(item, field.name.split(".")) }))
      .filter(({ value }) => scalar(value) && value !== null)
      .slice(0, 6)
  ).filter((sample) => sample.length);
  if (!samples.length) return undefined;

  const title = grounding.operation.summary?.trim() ||
    normalize(grounding.operation.path);
  const heading = `**${title}**${count === undefined ? "" : `: ${count} record${count === 1 ? "" : "s"} returned`}.`;
  const details = samples.map((sample, index) => {
    const label = samples.length > 1 ? `Sample ${index + 1}\n` : "";
    return `${label}${sample.map((candidate) => `- ${renderApiField(candidate)}`).join("\n")}`;
  });
  return renderApiAnswer([heading, ...details].join("\n\n"), grounding.operation);
}

/**
 * Render scalar fields selected directly from OpenAPI names and descriptions.
 * Equal-scoring fields are returned together rather than asking the model to
 * choose, which is both faster and safer for questions such as "which block?".
 * Field names, parameter names, and units all come from OpenAPI.
 * @param {{ question: string, data: unknown, arguments?: Record<string, unknown>, operation: { method: string, path: string, summary?: string, parameters?: { name: string }[], response: { type?: string, fields?: { name: string, type: string, description?: string, ownDescription?: string }[] } } }} grounding
 */
export function renderDirectApiAnswer(grounding) {
  const responseFields = grounding.operation.response.fields ?? [];
  const normalizedQuestion = normalize(grounding.question);
  const totalNoun =
    normalizedQuestion.match(/\btotal\s+([a-z0-9]+)\b/)?.[1] ??
    normalizedQuestion.match(/\b([a-z0-9]+)\s+(?:in\s+)?total\b/)?.[1];
  const directTotalFields = totalNoun
    ? responseFields.filter((field) => {
        const document = new Set(
          normalize(`${field.name} ${field.ownDescription ?? field.description ?? ""}`)
            .split(" "),
        );
        return document.has("total") &&
          [...document].some((word) => tokenAffinity(word, totalNoun) >= MIN_FIELD_AFFINITY);
      })
    : [];
  if (
    /\b(?:add(?:ed)?|altogether|combined?|difference|minus|net|plus|subtract(?:ed)?|sum)\b/i
      .test(grounding.question) ||
    totalNoun && directTotalFields.length !== 1
  ) return undefined;

  const data = focusApiData(grounding.data, grounding.arguments);
  if (scalar(data) && !responseFields.length) {
    const type = grounding.operation.response.type ?? "value";
    const label = normalize(type) || "value";
    return renderApiAnswer(`**${label}**: ${displayScalar(data)}`, grounding.operation);
  }

  const words = new Set(
    [...apiRequestWords(grounding.question)].filter((word) => word.length >= 3),
  );
  if (normalize(grounding.question).includes("how many")) {
    words.add("count");
    words.add("number");
  }
  const parameters = new Set(
    (grounding.operation.parameters ?? []).map(({ name }) => normalize(name)),
  );
  const asksForIdentity = words.has("which") || words.has("where");
  const fields = responseFields.map((field, index) => {
    const nameTokens = new Set(normalize(field.name).match(/[a-z0-9]+/g) ?? []);
    const ownTokens = new Set(
      normalize(field.ownDescription ?? field.description ?? "").match(/[a-z0-9]+/g) ?? [],
    );
    const tokens = new Set(
      normalize(`${field.name} ${field.description ?? ""}`).match(/[a-z0-9]+/g) ?? [],
    );
    return { field, index, nameTokens, ownTokens, tokens };
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
        const match = [...field.tokens]
          .map((token) => ({
            token,
            affinity: tokenAffinity(word, token),
          }))
          .sort((left, right) => right.affinity - left.affinity)[0];
        if (!match || match.affinity < MIN_FIELD_AFFINITY) continue;
        matches += 1;
        const frequency = frequencies.get(match.token) ?? fields.length;
        const idf = Math.log((fields.length + 1) / (frequency + 1)) + 1;
        const nameMatch = [...field.nameTokens].some((token) =>
          tokenAffinity(word, token) >= MIN_FIELD_AFFINITY
        );
        const ownMatch = [...field.ownTokens].some((token) =>
          tokenAffinity(word, token) >= MIN_FIELD_AFFINITY
        );
        score += idf * (nameMatch ? 3 : ownMatch ? 2 : 1) * match.affinity;
      }
      if (
        asksForIdentity &&
        normalize(field.field.type).split(" ").includes("boolean")
      ) score *= 0.5;
      return {
        field: field.field,
        index: field.index,
        path,
        value: valueAt(data, path),
        matches,
        score,
        explicitNameMatches: [...words].filter((word) =>
          [...field.nameTokens].some((token) =>
            tokenAffinity(word, token) >= MIN_FIELD_AFFINITY
          )
        ).length,
        parameter: parameters.has(normalize(leaf)),
        unmatchedName: [...field.nameTokens].filter((token) =>
          ![...words].some((word) =>
            tokenAffinity(word, token) >= MIN_FIELD_AFFINITY
          )
        ).length,
      };
    })
    .filter(({ matches, parameter, value }) =>
      matches > 0 && !parameter && scalar(value) && value !== null
    )
    .sort((left, right) =>
      right.score - left.score ||
      right.matches - left.matches ||
      left.unmatchedName - right.unmatchedName
    );
  if (!candidates.length) return renderApiOverview(grounding);

  const explicit = candidates.filter(({ explicitNameMatches }) => explicitNameMatches > 0);
  const selected = (
    /\band\b/i.test(grounding.question) && explicit.length > 1
      ? explicit
      : candidates.filter(({ score, matches, unmatchedName }) =>
          score === candidates[0].score &&
          matches === candidates[0].matches &&
          (matches === 1 || unmatchedName === candidates[0].unmatchedName)
        )
  )
    .slice(0, 6)
    .sort((left, right) => left.index - right.index);
  const answer = selected.length === 1
    ? renderApiField(selected[0])
    : selected.map((candidate) => `- ${renderApiField(candidate)}`).join("\n");
  return renderApiAnswer(answer, grounding.operation);
}
