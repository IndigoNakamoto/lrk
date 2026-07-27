import { normalize } from "../text.js";

const CHART_REQUEST =
  /\b(?:chart|graph|plot|trend|visualize|visualise)\b|\b(?:over|through)\s+time\b|\btime\s+series\b/;
const ADD_REQUEST = /\b(?:add|include|overlay|put)\b/;
const REMOVE_REQUEST = /\b(?:remove|drop)\b|\btake\b.+\boff\b/;
const KEEP_REQUEST = /\b(?:only\s+keep|keep\s+only)\b/;
const UNSUPPORTED_EDIT = /\b(?:clear|replace|reset|swap)\b/;
/** @type {[string, RegExp][]} */
const VIEW_REQUESTS = [
  ["stacked", /\b(?:stack|stacked)\b/],
  ["area", /\barea\b/],
  ["bar", /\b(?:bar|bars)\b/],
  ["dots", /\b(?:dot|dots|points?)\b/],
  ["line", /\b(?:line|lines)\b/],
];

/**
 * @param {string} request
 * @param {boolean} hasActiveChart
 */
export function directChartCommand(request, hasActiveChart) {
  const text = normalize(request);
  const scale = /\b(?:log|logarithmic)\b/.test(text)
    ? "log"
    : /\blinear\b/.test(text)
      ? "linear"
      : undefined;

  if (hasActiveChart) {
    const view = VIEW_REQUESTS.find(([, pattern]) => pattern.test(text))?.[0];
    if (scale || view) {
      return {
        kind: /** @type {const} */ ("style"),
        ...(scale ? { scale } : {}),
        ...(view ? { view } : {}),
      };
    }
  }
  if (!hasActiveChart && scale) {
    return { kind: /** @type {const} */ ("missing") };
  }
  if (!hasActiveChart && ADD_REQUEST.test(text) && !CHART_REQUEST.test(text)) {
    return { kind: /** @type {const} */ ("missing_add") };
  }

  if (hasActiveChart && ADD_REQUEST.test(text)) {
    return { kind: /** @type {const} */ ("edit"), operation: /** @type {const} */ ("add") };
  }
  if (hasActiveChart && REMOVE_REQUEST.test(text)) {
    return { kind: /** @type {const} */ ("edit"), operation: /** @type {const} */ ("remove") };
  }
  if (hasActiveChart && KEEP_REQUEST.test(text)) {
    return { kind: /** @type {const} */ ("edit"), operation: /** @type {const} */ ("replace") };
  }
  if (
    CHART_REQUEST.test(text) &&
    !REMOVE_REQUEST.test(text) &&
    !KEEP_REQUEST.test(text) &&
    !UNSUPPORTED_EDIT.test(text) &&
    (!hasActiveChart || !ADD_REQUEST.test(text))
  ) {
    return { kind: /** @type {const} */ ("build"), operation: /** @type {const} */ ("add") };
  }
  return undefined;
}
