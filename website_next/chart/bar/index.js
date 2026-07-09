import { createLinePathData, formatCoordinate } from "../path.js";
import { createStackedSeries } from "../stacked/series.js";
import { clamp } from "../math.js";
import { appendSeriesPath } from "../series-path.js";

/** @param {StackedPoint[]} points */
function getBarWidth(points) {
  return points.length > 1
    ? Math.abs(points[1].plotX - points[0].plotX) * 0.8
    : 1;
}

/**
 * @param {ChartFrame} frame
 * @param {StackedPoint[]} points
 * @param {number} width
 */
function createBarPathData(frame, points, width) {
  return points
    .map(({ plotX, plotY0, plotY1 }) => {
      const left = clamp(plotX - width / 2, frame.left, frame.right - width);
      const right = left + width;
      const top = Math.min(plotY0, plotY1);
      const bottom = Math.max(plotY0, plotY1);

      return (
        `M${formatCoordinate(left)} ${formatCoordinate(top)}` +
        `H${formatCoordinate(right)}V${formatCoordinate(bottom)}` +
        `H${formatCoordinate(left)}Z`
      );
    })
    .join(" ");
}

/**
 * @param {PlotContext} context
 */
export function renderBarPlot({
  group,
  loadedSeries,
  frame,
  highlight,
  scale,
  order,
}) {
  const { lineIndexes, plottedSeries, stackIndexes } = createStackedSeries(
    loadedSeries,
    frame,
    order,
    scale,
  );

  for (const index of stackIndexes) {
    const { color, points } = plottedSeries[index];
    appendSeriesPath({
      group,
      highlight,
      index,
      chart: "bar",
      color,
      d: createBarPathData(frame, points, getBarWidth(points)),
    });
  }

  for (const index of lineIndexes) {
    const { color, points } = plottedSeries[index];
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
