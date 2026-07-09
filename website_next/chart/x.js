import { getPlotWidth, insetPlotX } from "./viewbox.js";

/**
 * @param {ChartFrame} frame
 * @param {number} count
 */
export function createStepXScale(frame, count) {
  const width = getPlotWidth(frame);
  const last = count - 1;

  return /** @param {ChartX} _x @param {number} index */ (_x, index) =>
    insetPlotX(frame, last > 0 ? (index / last) * width : width / 2);
}

/**
 * @param {ChartFrame} frame
 * @param {number[]} values
 */
export function createLinearXScale(frame, values) {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min;
  const width = getPlotWidth(frame);

  return /** @param {ChartX} x */ (x) => {
    const value = /** @type {number} */ (x);

    return insetPlotX(
      frame,
      span ? ((value - min) / span) * width : width / 2,
    );
  };
}
