import { createSeriesHighlight } from "../highlight.js";
import { createLegend } from "../legend/index.js";
import { getChartPointRadius, layoutChartMarker } from "../marker.js";
import { createLinePathData } from "../path.js";
import { createChartPointer } from "../pointer.js";
import { createSvgElement } from "../svg.js";
import { createChartFrame, VIEWBOX_WIDTH } from "../viewbox.js";

/**
 * @param {Object} args
 * @param {string} args.title
 * @param {ChartUnit} args.unit
 * @param {string} args.ariaLabel
 * @param {number} args.fallbackHeight
 * @param {ChartFrameOptions} [args.gutter]
 * @param {XySeries[]} args.series
 * @param {(frame: ChartFrame) => XyPlottedSeries[]} args.plot
 * @param {false | ((series: XySeries, plotted: XyPlottedSeries, point: ChartPoint) => string)} [args.marker]
 */
export function createXyChart({
  title,
  unit,
  ariaLabel,
  fallbackHeight,
  gutter,
  series,
  plot,
  marker,
}) {
  const frameOptions =
    gutter ?? (marker === false ? { topPadding: 0 } : {});
  const figure = document.createElement("figure");
  const plotElement = document.createElement("div");
  const svg = createSvgElement("svg");
  const group = createSvgElement("g");
  const guide = createSvgElement("line");
  const markerElement = document.createElement("div");
  const { legend, menu, items, readout } = createLegend({
    title,
    unit,
    series,
  });
  const highlight = createSeriesHighlight(items, menu);
  const resizeObserver = new ResizeObserver(render);
  /** @type {XyPlottedSeries[]} */
  let currentSeries = [];
  /** @type {ChartFrame | undefined} */
  let currentFrame;
  let renderFrame = 0;
  const pointer = createChartPointer(
    svg,
    () => currentFrame?.height,
    ({ x, y }) => {
      const frame = currentFrame;
      if (!frame) return;

      const closest = findClosestPoint(series, currentSeries, x, y);

      if (!closest) {
        hideMarker();
        return;
      }

      showMarker(closest, frame);
    },
  );

  figure.dataset.chart = "xy";
  figure.dataset.chartLegend = "";
  plotElement.dataset.chart = "plot";
  guide.dataset.chart = "xy-guide";
  markerElement.dataset.chartMarker = "xy";
  svg.setAttribute("viewBox", `0 0 ${VIEWBOX_WIDTH} ${fallbackHeight}`);
  svg.setAttribute("role", "img");
  svg.setAttribute("aria-label", ariaLabel);
  svg.append(guide, group);
  plotElement.append(svg, markerElement);
  figure.append(legend, plotElement);

  function render() {
    const frame = createChartFrame(svg, fallbackHeight, frameOptions);
    const plottedSeries = plot(frame);
    const radius = getChartPointRadius(svg.getBoundingClientRect().width);

    svg.setAttribute("viewBox", `0 0 ${VIEWBOX_WIDTH} ${frame.height}`);
    currentFrame = frame;
    currentSeries = plottedSeries;
    highlight.clearNodes();
    group.replaceChildren();
    hideMarker();
    updateReadout(readout, unit, plottedSeries);

    plottedSeries.forEach((plotted, index) => {
      const item = series[index];

      if (item.kind === "line") {
        appendLine(group, highlight, item, plotted, index);
      } else {
        appendPoints(group, highlight, item, plotted, index, radius);
      }
    });
  }

  /**
   * @param {{ index: number, point: ChartPoint }} closest
   * @param {ChartFrame} frame
   */
  function showMarker(closest, frame) {
    const plotted = currentSeries[closest.index];
    const item = series[closest.index];

    guide.setAttribute("x1", closest.point.plotX.toFixed(2));
    guide.setAttribute("x2", closest.point.plotX.toFixed(2));
    guide.setAttribute("y1", "0");
    guide.setAttribute("y2", frame.bottom.toString());
    const text =
      marker === false
        ? ""
        : (marker?.(item, plotted, closest.point) ??
          unit.format(closest.point.y));

    markerElement.textContent = text;
    markerElement.hidden = !text;
    if (text) layoutChartMarker(markerElement, closest.point.plotX);
    svg.dataset.xyHover = "true";
    highlight.preview(closest.index);
  }

  function hideMarker() {
    delete svg.dataset.xyHover;
    markerElement.textContent = "";
    markerElement.hidden = true;
    highlight.clearPreview();
  }

  function disconnect() {
    cancelAnimationFrame(renderFrame);
    renderFrame = 0;
    pointer.cancel();
    resizeObserver.disconnect();
  }

  render();
  renderFrame = requestAnimationFrame(() => {
    renderFrame = 0;
    render();
  });
  resizeObserver.observe(svg);
  svg.addEventListener("pointerenter", pointer.measure);
  svg.addEventListener("pointermove", pointer.update);
  svg.addEventListener("pointerleave", () => {
    pointer.cancel();
    hideMarker();
  });

  return /** @type {const} */ ({
    destroy: disconnect,
    element: figure,
  });
}

/**
 * @param {XySeries[]} series
 * @param {XyPlottedSeries[]} plottedSeries
 * @param {number} x
 * @param {number} y
 */
function findClosestPoint(series, plottedSeries, x, y) {
  /** @type {{ index: number, point: ChartPoint } | null} */
  let closest = null;
  let closestDistance = Infinity;

  plottedSeries.forEach((item, index) => {
    if (series[index].hidden) return;

    for (const point of item.points) {
      const distance = Math.hypot(point.plotX - x, point.plotY - y);

      if (distance < closestDistance) {
        closest = { index, point };
        closestDistance = distance;
      }
    }
  });

  return closest;
}

/**
 * @param {LegendReadout} readout
 * @param {ChartUnit} unit
 * @param {XyPlottedSeries[]} plottedSeries
 */
function updateReadout(readout, unit, plottedSeries) {
  readout.rows.forEach((row, index) => {
    if (!row) return;

    const value = plottedSeries[index]?.readout;

    row.value.textContent =
      typeof value === "number" ? unit.format(value) : (value ?? "");
  });
}

/**
 * @param {SVGGElement} group
 * @param {SeriesHighlight} highlight
 * @param {XySeries} series
 * @param {XyPlottedSeries} plotted
 * @param {number} index
 */
function appendLine(group, highlight, series, plotted, index) {
  const path = createSvgElement("path");

  path.dataset.chart = "xy-line";
  path.dataset.series = index.toString();
  path.style.setProperty("--color", series.color());
  path.setAttribute("d", createLinePathData(plotted.points));
  highlight.addNode(path, index);
  group.append(path);
}

/**
 * @param {SVGGElement} group
 * @param {SeriesHighlight} highlight
 * @param {XySeries} series
 * @param {XyPlottedSeries} plotted
 * @param {number} index
 * @param {number} radius
 */
function appendPoints(group, highlight, series, plotted, index, radius) {
  for (const point of plotted.points) {
    const circle = createSvgElement("circle");

    circle.dataset.chart = "xy-point";
    circle.dataset.series = index.toString();
    circle.style.setProperty("--color", series.color());
    circle.setAttribute("cx", point.plotX.toFixed(2));
    circle.setAttribute("cy", point.plotY.toFixed(2));
    circle.setAttribute("r", radius.toString());
    highlight.addNode(circle, index);
    group.append(circle);
  }
}
