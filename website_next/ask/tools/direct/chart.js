import { normalize } from "../text.js";

const CHART_REQUEST =
  /\b(?:chart|graph|plot|trend|visualize|visualise)\b|\b(?:over|through)\s+time\b|\btime\s+series\b/;
const ADD_REQUEST = /\b(?:add|include|overlay|put)\b/;
const REMOVE_REQUEST = /\b(?:remove|drop)\b|\btake\b.+\boff\b/;
const KEEP_REQUEST = /\b(?:only\s+keep|keep\s+only)\b/;
const UNSUPPORTED_EDIT = /\b(?:clear|replace|reset|swap)\b/;

/**
 * @param {string} request
 * @param {boolean} hasActiveChart
 */
export function directChartCommand(request, hasActiveChart) {
  const text = normalize(request);

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
