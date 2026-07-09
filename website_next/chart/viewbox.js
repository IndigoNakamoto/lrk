import { CHART_FRAME, CHART_MARKER, CHART_SIZE } from "./constants.js";

export const VIEWBOX_WIDTH = CHART_SIZE.width;
export const FALLBACK_VIEWBOX_HEIGHT = CHART_SIZE.fallbackHeight;

/**
 * @param {SVGSVGElement} svg
 * @param {number} [fallbackHeight]
 */
export function getViewBoxHeight(svg, fallbackHeight = FALLBACK_VIEWBOX_HEIGHT) {
  const { width, height } = svg.getBoundingClientRect();

  return width && height ? (VIEWBOX_WIDTH * height) / width : fallbackHeight;
}

/**
 * @param {SVGSVGElement} svg
 * @param {number} height
 */
function getViewBoxUnit(svg, height) {
  return svg.clientHeight ? height / svg.clientHeight : 1;
}

/**
 * @param {SVGSVGElement} svg
 * @param {number} [fallbackHeight]
 * @param {ChartFrameOptions} [options]
 * @returns {ChartFrame}
 */
export function createChartFrame(
  svg,
  fallbackHeight = FALLBACK_VIEWBOX_HEIGHT,
  options = {},
) {
  const height = getViewBoxHeight(svg, fallbackHeight);
  const unit = getViewBoxUnit(svg, height);
  const leftPadding = options.leftPadding ?? 0;
  const rightPadding = options.rightPadding ?? 0;
  const topPadding =
    options.topPadding ?? CHART_MARKER.height + CHART_FRAME.topGap;
  const bottomPadding = options.bottomPadding ?? CHART_FRAME.bottomPadding;
  const left = leftPadding * unit;
  const right = Math.max(left + 1, VIEWBOX_WIDTH - rightPadding * unit);
  const top = topPadding * unit;
  const bottom = Math.max(top + 1, height - bottomPadding * unit);

  return {
    width: VIEWBOX_WIDTH,
    height,
    left,
    right,
    top,
    bottom,
    plotWidth: right - left,
    plotHeight: bottom - top,
  };
}

/** @param {ChartFrame} frame */
export function getPlotWidth(frame) {
  return frame.plotWidth;
}

/** @param {ChartFrame} frame */
export function getPlotHeight(frame) {
  return frame.plotHeight;
}

/** @param {ChartFrame} frame */
export function getPlotBottom(frame) {
  return frame.bottom;
}

/**
 * @param {ChartFrame} frame
 * @param {number} x
 */
export function insetPlotX(frame, x) {
  return frame.left + x;
}

/**
 * @param {ChartFrame} frame
 * @param {number} y
 */
export function insetPlotY(frame, y) {
  return frame.top + y;
}
