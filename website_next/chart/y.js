import { scaleY } from "./scale.js";
import { getPlotHeight, insetPlotY } from "./viewbox.js";

/**
 * @param {ChartFrame} frame
 * @param {ScaleBounds} bounds
 * @param {ChartScale} scale
 */
export function createYScale(frame, bounds, scale) {
  const height = getPlotHeight(frame);

  return /** @param {number} y */ (y) =>
    insetPlotY(frame, scaleY(y, bounds, height, scale));
}

/**
 * @param {ChartFrame} frame
 * @param {number[]} values
 * @param {(value: number) => number} [transform]
 */
export function createValueYScale(frame, values, transform = (value) => value) {
  const scaledValues = values.map(transform);
  const min = Math.min(...scaledValues);
  const max = Math.max(...scaledValues);
  const span = max - min;
  const height = getPlotHeight(frame);

  return /** @param {number} y */ (y) =>
    insetPlotY(
      frame,
      span ? (1 - (transform(y) - min) / span) * height : height / 2,
    );
}
