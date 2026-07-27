import { normalize } from "../text.js";
import { searchMetrics } from "./index.js";
import { canonicalMetricQuery } from "./language.js";

export const MAX_OPTIONS = 12;

/** @param {string[]} topics @param {() => void} onProgress */
export async function exactTopicMetrics(topics, onProgress) {
  if (!topics.length) return [];

  const names = new Set(topics.map(normalize));
  const metrics = await searchMetrics(topics, MAX_OPTIONS, [], onProgress);
  return metrics.filter((metric) => names.has(normalize(metric.name)));
}

/**
 * Resolve coordinated metric wording only when expanding its shared suffix
 * produces two exact generated catalog names.
 * @param {string} request
 * @param {() => void} onProgress
 */
export async function coordinatedMetrics(request, onProgress) {
  const expression = canonicalMetricQuery(request)
    .replace(
      /^(?:(?:compare|chart|graph|plot|show|visualize|visualise)(?: me)?|comparison of)\s+/,
      "",
    )
    .replace(/^both\s+/, "")
    .trim();
  const parts = expression
    .split(/\s+(?:and|against|versus|vs)\s+/)
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length !== 2) return [];

  const [left, right] = parts;
  const candidates = [[left, right]];
  const leftWords = left.split(" ");
  const rightWords = right.split(" ");
  for (let index = 1; index < rightWords.length; index += 1) {
    candidates.push([`${left} ${rightWords.slice(index).join(" ")}`, right]);
  }
  for (let index = 1; index < leftWords.length; index += 1) {
    candidates.push([left, `${right} ${leftWords.slice(index).join(" ")}`]);
  }

  for (const topics of candidates) {
    const metrics = await exactTopicMetrics(topics, onProgress);
    if (new Set(metrics.map((metric) => metric.path)).size === 2) return metrics;
  }
  return [];
}

/** @param {any[]} items */
export function uniqueMetricOptions(items) {
  return [...new Map(items.map((/** @type {any} */ item) => [item.ref, item])).values()];
}

/** @param {any[][]} groups */
export function mergeMetricGroups(groups) {
  const output = [];
  const positions = new Map();
  const ranks = Math.max(...groups.map((group) => group.length), 0);

  for (let rank = 0; rank < ranks && output.length < MAX_OPTIONS; rank += 1) {
    for (const group of groups) {
      const metric = group[rank];
      if (!metric) continue;

      const position = positions.get(metric.path);
      if (position !== undefined) {
        const current = output[position];
        const exact = normalize(metric.name) === normalize(metric.matchedQuery ?? "");
        const currentExact = normalize(current.name) === normalize(current.matchedQuery ?? "");
        if (exact && !currentExact) output[position] = metric;
        continue;
      }
      positions.set(metric.path, output.length);
      output.push(metric);
      if (output.length === MAX_OPTIONS) break;
    }
  }
  return output;
}

/** @param {any[]} items */
export function balancedOptions(items) {
  const order = ["fact", "guide", "metric", "source"];
  const groups = order
    .map((kind) => items
      .filter((item) => item.kind === kind)
      .sort((left, right) => right.score - left.score))
    .filter((group) => group.length);
  const output = [];

  for (let rank = 0; output.length < MAX_OPTIONS; rank += 1) {
    let added = false;
    for (const group of groups) {
      const item = group[rank];
      if (!item) continue;
      const { score, ...option } = item;
      output.push(option);
      added = true;
      if (output.length === MAX_OPTIONS) break;
    }
    if (!added) break;
  }
  return output;
}
