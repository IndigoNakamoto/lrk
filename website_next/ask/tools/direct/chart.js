import { normalize } from "../text.js";

const CHART_REQUEST =
  /\b(?:chart|graph|plot|trend|visualize|visualise)\b|\b(?:over|through)\s+time\b|\btime\s+series\b/;
const ADD_REQUEST = /\b(?:add|include|overlay)\b/;
const REMOVE_REQUEST = /\b(?:remove|drop)\b/;

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
  if (CHART_REQUEST.test(text) && !ADD_REQUEST.test(text) && !REMOVE_REQUEST.test(text)) {
    return { kind: /** @type {const} */ ("build"), operation: /** @type {const} */ ("add") };
  }
  return undefined;
}
