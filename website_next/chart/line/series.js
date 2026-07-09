import { interpolatePlotValue } from "../interpolate.js";
import { createChartPoints } from "../points.js";
import { createBounds, includeBoundValue } from "../scale.js";
import { createStepXScale } from "../x.js";
import { createYScale } from "../y.js";

/** @param {LoadedSeries[]} series */
function createValueBounds(series) {
  const bounds = createBounds();

  for (const { samples } of series) {
    for (const { y } of samples) {
      includeBoundValue(bounds, y);
    }
  }

  return bounds;
}

/**
 * @param {ChartSample[]} samples
 * @param {ScaleBounds} bounds
 * @param {ChartFrame} frame
 * @param {ChartScale} scale
 * @returns {ChartPoint[]}
 */
function createPoints(samples, bounds, frame, scale) {
  const scaleX = createStepXScale(frame, samples.length);
  const scalePlotY = createYScale(frame, bounds, scale);

  return createChartPoints(samples, scaleX, scalePlotY);
}

/**
 * @param {LoadedSeries[]} loadedSeries
 * @param {ChartFrame} frame
 * @param {ChartScale} scale
 */
export function createLineSeries(loadedSeries, frame, scale) {
  const bounds = createValueBounds(loadedSeries);

  return loadedSeries.map(({ series, color, samples }) => {
    const points = createPoints(samples, bounds, frame, scale);

    return {
      series,
      color,
      points,
      hitTest: /** @type {PlottedSeries["hitTest"]} */ (
        (_point, pointerX, pointerY) =>
          Math.abs(
            interpolatePlotValue(points, pointerX, ({ plotY }) => plotY) -
              pointerY,
          )
      ),
    };
  });
}
