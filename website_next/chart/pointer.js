import { VIEWBOX_WIDTH } from "./viewbox.js";

/**
 * @typedef {Object} ChartPointerPosition
 * @property {number} x
 * @property {number} y
 */

/**
 * @param {SVGSVGElement} svg
 * @param {() => number | undefined} getHeight
 * @param {(position: ChartPointerPosition) => void} onMove
 */
export function createChartPointer(svg, getHeight, onMove) {
  let rect = svg.getBoundingClientRect();
  let clientX = 0;
  let clientY = 0;
  let frame = 0;

  function measure() {
    rect = svg.getBoundingClientRect();

    return rect;
  }

  function cancel() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }

  /** @param {PointerEvent} event */
  function update(event) {
    clientX = event.clientX;
    clientY = event.clientY;
    if (frame) return;

    frame = requestAnimationFrame(() => {
      frame = 0;
      const height = getHeight();
      if (height === undefined) return;

      onMove({
        x: ((clientX - rect.left) / rect.width) * VIEWBOX_WIDTH,
        y: ((clientY - rect.top) / rect.height) * height,
      });
    });
  }

  return /** @type {const} */ ({
    cancel,
    measure,
    update,
  });
}
