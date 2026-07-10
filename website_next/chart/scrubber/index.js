import { clamp } from "../math.js";
import {
  getChartPointRadius,
  layoutChartMarker,
} from "../marker.js";
import { createChartPointer } from "../pointer.js";
import { createSvgElement } from "../svg.js";
import { getPlotBottom } from "../viewbox.js";

const dateFormat = new Intl.DateTimeFormat("en-US", {
  day: "2-digit",
  month: "2-digit",
  year: "numeric",
});

/**
 * @param {ScrubberSeries} series
 * @param {number} step
 */
function getPointAtStep(series, step) {
  return series.points[step];
}

/**
 * @param {ScrubberSeries[]} series
 * @param {ChartPoint[]} points
 * @param {number} x
 * @param {number} y
 */
function getClosestPointIndex(series, points, x, y) {
  let closestIndex = 0;
  let closestDistance = Infinity;

  for (const [index, point] of points.entries()) {
    const distance =
      series[index].hitTest?.(point, x, y) ?? Math.abs(point.plotY - y);

    if (distance < closestDistance) {
      closestIndex = index;
      closestDistance = distance;
    }
  }

  return closestIndex;
}

/**
 * @param {LegendReadout} readout
 * @param {ChartPoint[]} points
 * @param {(value: number) => string} format
 */
function updateReadout(readout, points, format) {
  readout.rows.forEach((row, index) => {
    if (!row) return;
    row.value.textContent = format(points[index].y);
  });
}

/**
 * @param {SVGSVGElement} svg
 * @param {LegendReadout} readout
 * @param {SeriesHighlight} highlight
 * @param {(value: number) => string} format
 */
export function createScrubber(svg, readout, highlight, format) {
  const group = createSvgElement("g");
  const shade = createSvgElement("rect");
  const guide = createSvgElement("line");
  const plot = /** @type {HTMLElement} */ (svg.parentElement);
  const dateMarker = document.createElement("div");
  /** @type {ScrubberSeries[]} */
  let series = [];
  /** @type {SVGCircleElement[]} */
  let markers = [];
  /** @type {ChartFrame | undefined} */
  let frame;
  let stepCount = 0;
  let currentStep = -1;
  /** @type {ChartPoint[]} */
  let currentPoints = [];
  const pointer = createChartPointer(
    svg,
    () => frame?.height,
    ({ x, y }) => {
      const currentFrame = frame;
      if (!currentFrame) return;

      update((x - currentFrame.left) / currentFrame.plotWidth, x, y);
    },
  );

  group.dataset.scrubber = "root";
  shade.dataset.scrubber = "shade";
  guide.dataset.scrubber = "guide";
  dateMarker.dataset.chartMarker = "date";
  group.append(shade, guide);
  svg.append(group);
  plot.append(dateMarker);

  /** @param {number} step */
  function getPointsAtStep(step) {
    return series.map((item) => getPointAtStep(item, step));
  }

  /**
   * @param {number} ratio
   * @param {number} [x]
   * @param {number} [y]
   * @param {boolean} [scrubbing]
   */
  function update(ratio, x, y, scrubbing = true) {
    if (!series.length || !frame) return;

    const nextStep = Math.round(clamp(ratio, 0, 1) * stepCount);

    if (nextStep !== currentStep) {
      currentStep = nextStep;
      currentPoints = getPointsAtStep(nextStep);

      const plotX = currentPoints[0].plotX;
      const xText = plotX.toFixed(2);
      const plotBottom = getPlotBottom(frame);

      svg.dataset.index = nextStep.toString();
      shade.setAttribute("x", xText);
      shade.setAttribute("y", "0");
      shade.setAttribute("width", (frame.right - plotX).toFixed(2));
      shade.setAttribute("height", plotBottom.toString());
      guide.setAttribute("x1", xText);
      guide.setAttribute("x2", xText);
      guide.setAttribute("y1", "0");
      guide.setAttribute("y2", plotBottom.toString());
      dateMarker.textContent = dateFormat.format(
        /** @type {Date} */ (currentPoints[0].x),
      );
      dateMarker.setAttribute(
        "aria-label",
        `Date ${dateMarker.textContent}`,
      );
      layoutChartMarker(dateMarker, plotX);
      updateReadout(readout, currentPoints, format);

      markers.forEach((marker, index) => {
        const point = currentPoints[index];

        marker.setAttribute("cx", point.plotX.toFixed(2));
        marker.setAttribute("cy", point.plotY.toFixed(2));
      });
    }

    if (scrubbing) {
      svg.dataset.scrubbing = "true";
    } else {
      delete svg.dataset.scrubbing;
    }

    if (x !== undefined && y !== undefined) {
      highlight.preview(getClosestPointIndex(series, currentPoints, x, y));
    }
  }

  function hide() {
    update(1, undefined, undefined, false);
  }

  function clear() {
    pointer.cancel();
    series = [];
    markers = [];
    currentStep = -1;
    currentPoints = [];
    frame = undefined;
    dateMarker.style.display = "none";
    highlight.clearPreview();
    group.replaceChildren(shade, guide);
    delete svg.dataset.index;
    delete svg.dataset.scrubbing;
  }

  /**
   * @param {ScrubberSeries[]} nextSeries
   * @param {ChartFrame} nextFrame
   */
  function setSeries(nextSeries, nextFrame) {
    series = nextSeries;
    frame = nextFrame;
    currentStep = -1;
    stepCount = Math.max(...series.map(({ points }) => points.length - 1));
    const { width } = pointer.measure();
    dateMarker.style.display = "";
    const radius = getChartPointRadius(width);
    markers = series.map(({ color }, index) => {
      const marker = createSvgElement("circle");

      marker.dataset.series = index.toString();
      marker.dataset.scrubber = "marker";
      marker.style.setProperty("--color", color);
      marker.setAttribute("r", radius.toString());
      highlight.addNode(marker, index);

      return marker;
    });

    group.replaceChildren(shade, guide, ...markers);
    update(1, undefined, undefined, false);
  }

  svg.addEventListener("pointerenter", pointer.measure);
  svg.addEventListener("pointermove", pointer.update);
  svg.addEventListener("pointerleave", () => {
    pointer.cancel();
    highlight.clearPreview();
    hide();
  });
  svg.addEventListener("focus", () => update(1));
  svg.addEventListener("blur", () => {
    pointer.cancel();
    highlight.clearPreview();
    hide();
  });
  svg.addEventListener("keydown", (event) => {
    const current = Number(svg.dataset.index || stepCount);

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      update((current - 1) / stepCount);
    }

    if (event.key === "ArrowRight") {
      event.preventDefault();
      update((current + 1) / stepCount);
    }
  });

  return { clear, setSeries };
}

/**
 * @typedef {Object} ScrubberSeries
 * @property {string} color
 * @property {ChartPoint[]} points
 * @property {PlottedSeries["hitTest"]} [hitTest]
 */
