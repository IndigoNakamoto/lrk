import { formatValue } from "./data.js";

/**
 * @typedef {Object} MetricRead
 * @property {string} label
 * @property {string | undefined} unit
 * @property {string} index
 * @property {number | string} start
 * @property {string | undefined} stamp
 * @property {unknown[]} values
 */

/** @param {{ facts: string[], sources: string[], excerpts: { path: string, startLine: number, content: string }[] }} evidence */
export function renderEvidence(evidence) {
  const sections = [...new Set(evidence.facts)].filter(Boolean);
  for (const excerpt of evidence.excerpts) {
    sections.push(`\`${excerpt.path}:${excerpt.startLine}\`\n\n\`\`\`\n${excerpt.content}\n\`\`\``);
  }
  const sources = [...new Set(evidence.sources)].filter(
    (source) => !sections.some((section) => section.includes(source)),
  );
  if (sources.length) {
    sections.push(`Source${sources.length === 1 ? "" : "s"}: ${sources.map((source) => `\`${source}\``).join(", ")}`);
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
