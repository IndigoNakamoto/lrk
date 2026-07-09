import { interpolatePlotValue } from "../interpolate.js";
import { orderIndexes } from "../order.js";
import { createBounds, includeBoundValue } from "../scale.js";
import { createStepXScale } from "../x.js";
import { createYScale } from "../y.js";

/**
 * @param {LoadedSeries[]} series
 * @param {number[]} stackOrder
 * @param {number[]} lineIndexes
 */
function createStackBounds(series, stackOrder, lineIndexes) {
  const bounds = createBounds();
  const length = series[0].samples.length;

  includeBoundValue(bounds, 0);

  for (let index = 0; index < length; index += 1) {
    let negative = 0;
    let positive = 0;

    for (const seriesIndex of stackOrder) {
      const y = series[seriesIndex].samples[index].y;
      const end = y < 0 ? negative + y : positive + y;

      if (y < 0) negative = end;
      else positive = end;

      includeBoundValue(bounds, end);
    }

    for (const seriesIndex of lineIndexes) {
      const y = series[seriesIndex].samples[index].y;

      includeBoundValue(bounds, y);
    }
  }

  return bounds;
}

/**
 * @param {LoadedSeries[]} loadedSeries
 * @param {ChartFrame} frame
 * @param {ChartOrder} order
 * @param {ChartScale} scale
 */
export function createStackedSeries(loadedSeries, frame, order, scale) {
  const indexes = loadedSeries.map((_, index) => index);
  const lineIndexes = orderIndexes(
    indexes.filter((index) => loadedSeries[index].series.role === "line"),
    order,
  );
  const stackIndexes = orderIndexes(
    indexes.filter((index) => loadedSeries[index].series.role !== "line"),
    order,
  );

  const length = loadedSeries[0].samples.length;
  const scaleX = createStepXScale(frame, length);
  const plottedSeries = loadedSeries.map(({ series, color }) => ({
    series,
    color,
    points: /** @type {StackedPoint[]} */ ([]),
    hitTest: /** @type {StackedPlottedSeries["hitTest"]} */ (undefined),
  }));

  const bounds = createStackBounds(loadedSeries, stackIndexes, lineIndexes);
  const scalePlotY = createYScale(frame, bounds, scale);

  for (const index of stackIndexes) {
    const points = plottedSeries[index].points;

    plottedSeries[index].hitTest = (_point, pointerX, pointerY) => {
      if (!points.length) return Infinity;

      const plotY0 = interpolatePlotValue(
        points,
        pointerX,
        (point) => point.plotY0,
      );
      const plotY1 = interpolatePlotValue(
        points,
        pointerX,
        (point) => point.plotY1,
      );
      const top = Math.min(plotY0, plotY1);
      const bottom = Math.max(plotY0, plotY1);

      return pointerY >= top && pointerY <= bottom
        ? 0
        : Math.min(Math.abs(pointerY - top), Math.abs(pointerY - bottom));
    };
  }

  for (const index of lineIndexes) {
    const points = plottedSeries[index].points;

    plottedSeries[index].hitTest = (_point, pointerX, pointerY) =>
      Math.abs(
        interpolatePlotValue(points, pointerX, ({ plotY }) => plotY) -
          pointerY,
      );
  }

  for (let index = 0; index < length; index += 1) {
    let negative = 0;
    let positive = 0;

    for (const seriesIndex of stackIndexes) {
      const { x, y } = loadedSeries[seriesIndex].samples[index];
      const start = y < 0 ? negative : positive;
      const end = start + y;
      const plotX = scaleX(x, index);

      if (y < 0) negative = end;
      else positive = end;

      const plotY0 = scalePlotY(start);
      const plotY1 = scalePlotY(end);

      plottedSeries[seriesIndex].points.push({
        x,
        y,
        plotX,
        plotY: plotY1,
        plotY0,
        plotY1,
      });
    }

    for (const seriesIndex of lineIndexes) {
      const { x, y } = loadedSeries[seriesIndex].samples[index];
      const plotY = scalePlotY(y);

      plottedSeries[seriesIndex].points.push({
        x,
        y,
        plotX: scaleX(x, index),
        plotY,
        plotY0: plotY,
        plotY1: plotY,
      });
    }
  }

  return {
    lineIndexes,
    plottedSeries,
    stackIndexes,
  };
}
