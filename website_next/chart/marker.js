import { CHART_MARKER, CHART_POINT, CHART_SIZE } from "./constants.js";

const VIEWBOX_WIDTH = CHART_SIZE.width;

/**
 * @param {HTMLElement} marker
 * @param {number} y
 */
export function sizeChartMarker(marker, y = 0) {
  marker.style.top = `${y}px`;
  marker.style.height = `${CHART_MARKER.height}px`;
}

/** @param {HTMLElement} marker */
function getMarkerWidth(marker) {
  return marker.offsetWidth || CHART_MARKER.fallbackWidth;
}

/**
 * @param {HTMLElement} marker
 * @param {number} xValue
 * @param {number} [viewWidth]
 */
export function positionChartMarker(marker, xValue, viewWidth = VIEWBOX_WIDTH) {
  const parentWidth = marker.parentElement?.clientWidth || viewWidth;
  const markerWidth = getMarkerWidth(marker);
  const x = (xValue / viewWidth) * parentWidth;
  const min = -CHART_MARKER.edgeOverflow;
  const max = Math.max(
    min,
    parentWidth - markerWidth + CHART_MARKER.edgeOverflow,
  );
  const left = Math.min(Math.max(x - markerWidth / 2, min), max);

  marker.style.left = `${left.toFixed(2)}px`;
}

/**
 * @param {HTMLElement} marker
 * @param {number} xValue
 */
export function layoutChartMarker(marker, xValue) {
  sizeChartMarker(marker);
  positionChartMarker(marker, xValue);
}

/** @param {number} viewWidth */
export function getChartPointRadius(viewWidth) {
  return viewWidth
    ? (CHART_POINT.radius * CHART_SIZE.width) / viewWidth
    : CHART_POINT.radius;
}
