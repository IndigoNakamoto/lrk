import { createAreaPathData, createLinePathData } from "../path.js";
import { appendSeriesPath } from "../series-path.js";
import { createOrderedIndexes } from "../order.js";
import { createLineSeries } from "../line/series.js";
import { getPlotBottom } from "../viewbox.js";

/**
 * @param {ChartFrame} frame
 * @param {ChartPoint[]} points
 * @returns {StackedPoint[]}
 */
function createAreaPoints(frame, points) {
  const bottom = getPlotBottom(frame);

  return points.map((point) => ({
    ...point,
    plotY0: bottom,
    plotY1: point.plotY,
  }));
}

/**
 * @param {PlotContext} context
 */
export function renderAreaPlot({
  group,
  loadedSeries,
  frame,
  highlight,
  scale,
  order,
}) {
  const plottedSeries = createLineSeries(loadedSeries, frame, scale);
  const indexes = createOrderedIndexes(plottedSeries.length, order);

  for (const index of indexes) {
    const { color, points } = plottedSeries[index];
    appendSeriesPath({
      group,
      highlight,
      index,
      chart: "area",
      color,
      d: createAreaPathData(createAreaPoints(frame, points)),
    });

    appendSeriesPath({
      group,
      highlight,
      index,
      chart: "line",
      color,
      d: createLinePathData(points),
    });
  }

  return plottedSeries;
}
