import { MAX_BLOCK_WEIGHT } from "../../format.js";
import { createPreviewFeeRange, orderTransactions } from "./fees.js";
import { createSquareLayout } from "./layout.js";
import { getCanvasFeeRateColor } from "./color.js";
import { createPreviewGeometry, hitTest } from "./geometry.js";
import { drawPreview } from "./draw.js";

const COLUMNS = 86;
const VISIBLE_CELLS = COLUMNS * COLUMNS;

/**
 * @param {BlockPreviewData} data
 * @param {Uint32Array} order
 * @param {number[]} ranges
 */
function createCapacityCells(data, order, ranges) {
  const count = Math.min(order.length, VISIBLE_CELLS);
  const colors = /** @type {Map<number, string>} */ (new Map());
  const cells = [];

  for (let index = 0; index < count; index += 1) {
    const offset = order[index];
    const feeRate = data.feeRates[offset];
    const weight = data.weights[offset];
    let color = colors.get(feeRate);

    if (color === undefined) {
      color = getCanvasFeeRateColor(feeRate, ranges);
      colors.set(feeRate, color);
    }

    cells.push({
      color,
      transaction: {
        feeRate,
        txIndex: data.range.start + offset,
        weight,
      },
      weight,
    });
  }

  return cells;
}

/**
 * @param {BlockPreviewData} data
 * @param {Object} [options]
 * @param {(transaction: BlockPreviewTransaction | null, point: BlockPreviewPointer | null, eager: boolean) => void} [options.onInspect]
 */
export function createBlockPreviewHeatmap(data, options = {}) {
  const canvas = document.createElement("canvas");
  const context = /** @type {CanvasRenderingContext2D} */ (
    canvas.getContext("2d")
  );
  const order = orderTransactions(data.weights, data.feeRates);
  const ranges = createPreviewFeeRange(data.feeRates, order);
  const cells = createCapacityCells(data, order, ranges);
  const square = createSquareLayout(
    cells,
    data.blockWeight,
    MAX_BLOCK_WEIGHT,
    COLUMNS,
  );
  const hiddenCounts = new Uint8Array(data.range.end - data.range.start);
  let hiddenFilterCount = 0;
  let frame = 0;
  let inspected = /** @type {BlockPreviewTransaction | null} */ (null);
  let inspectFrame = 0;
  let inspectPoint = /** @type {BlockPreviewPointer | null} */ (null);
  let bounds = /** @type {DOMRectReadOnly | null} */ (null);
  let previewMembership = /** @type {Uint8Array | null} */ (null);
  let geometry = /** @type {PreviewGeometry | null} */ (null);
  let rectWidth = 0;
  let capturedPointer = /** @type {number | null} */ (null);

  canvas.dataset.blockPreviewHeatmap = "";

  function measure() {
    bounds = canvas.getBoundingClientRect();

    return bounds;
  }

  function draw() {
    const width = measure().width;

    if (width <= 0) return;

    const dpr = globalThis.devicePixelRatio || 1;
    const size = Math.round(width * dpr);

    if (canvas.width !== size || canvas.height !== size) {
      canvas.width = size;
      canvas.height = size;
    }

    if (rectWidth !== width) {
      rectWidth = width;
      geometry = createPreviewGeometry(square, canvas, width);
    }

    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width, width);
    drawPreview({
      context,
      hasHidden: hiddenFilterCount > 0,
      hiddenCounts,
      inspected,
      previewMembership,
      rects: geometry?.rects ?? [],
      start: data.range.start,
    });
  }

  function scheduleDraw() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(draw);
  }

  function cancelInspectFrame() {
    cancelAnimationFrame(inspectFrame);
    inspectFrame = 0;
    inspectPoint = null;
  }

  /** @param {BlockPreviewTransaction | null} transaction */
  function setInspected(transaction) {
    if (transaction === inspected) return;

    inspected = transaction;
    scheduleDraw();
  }

  /**
   * @param {BlockPreviewPointer} point
   * @param {boolean} eager
   */
  function inspectAt(point, eager) {
    const nextBounds = bounds ?? measure();
    if (geometry === null) draw();
    if (geometry === null) return;

    const transaction = hitTest(
      geometry,
      point.clientX - nextBounds.left,
      point.clientY - nextBounds.top,
    );

    if (transaction === null && inspected !== null) {
      options.onInspect?.(inspected, point, eager);
      return;
    }

    setInspected(transaction);
    options.onInspect?.(transaction, point, eager);
  }

  /** @param {PointerEvent} event */
  function pointFromEvent(event) {
    return { clientX: event.clientX, clientY: event.clientY };
  }

  function inspectLatest() {
    const point = inspectPoint;

    inspectFrame = 0;
    inspectPoint = null;
    if (point !== null) inspectAt(point, false);
  }

  /** @param {PointerEvent} event */
  function scheduleInspect(event) {
    inspectPoint = pointFromEvent(event);
    if (inspectFrame) return;

    inspectFrame = requestAnimationFrame(inspectLatest);
  }

  /** @param {PointerEvent} event */
  function startInspect(event) {
    capturedPointer = event.pointerId;
    canvas.setPointerCapture(event.pointerId);
    cancelInspectFrame();
    measure();
    inspectAt(pointFromEvent(event), true);
    if (event.pointerType !== "mouse") event.preventDefault();
  }

  /** @param {PointerEvent} event */
  function moveInspect(event) {
    if (capturedPointer !== null || event.pointerType === "mouse") {
      scheduleInspect(event);
      if (capturedPointer !== null && event.pointerType !== "mouse") {
        event.preventDefault();
      }
    }
  }

  /** @param {PointerEvent} event */
  function stopInspect(event) {
    if (capturedPointer !== event.pointerId) return;

    capturedPointer = null;
    canvas.releasePointerCapture(event.pointerId);
    if (event.pointerType !== "mouse") event.preventDefault();
  }

  function clearInspect() {
    cancelInspectFrame();
    setInspected(null);
    options.onInspect?.(null, null, false);
  }

  /** @param {PointerEvent} event */
  function clearOnOutsidePointer(event) {
    if (event.target instanceof Node && canvas.contains(event.target)) return;

    clearInspect();
  }

  const observer = new ResizeObserver(scheduleDraw);

  canvas.addEventListener("pointerenter", measure);
  canvas.addEventListener("pointermove", moveInspect);
  canvas.addEventListener("pointerdown", startInspect);
  canvas.addEventListener("pointerup", stopInspect);
  canvas.addEventListener("pointercancel", (event) => {
    stopInspect(event);
    clearInspect();
  });
  canvas.addEventListener("pointerleave", (event) => {
    if (capturedPointer === null && event.pointerType === "mouse") {
      clearInspect();
    }
  });
  document.addEventListener("pointerdown", clearOnOutsidePointer);
  window.addEventListener("blur", clearInspect);
  observer.observe(canvas);
  scheduleDraw();

  return /** @type {const} */ ({
    element: canvas,
    destroy() {
      cancelAnimationFrame(frame);
      cancelInspectFrame();
      document.removeEventListener("pointerdown", clearOnOutsidePointer);
      window.removeEventListener("blur", clearInspect);
      observer.disconnect();
    },
    /** @param {Uint8Array | null} membership */
    setPreviewMembership(membership) {
      if (previewMembership === membership) return;

      previewMembership = membership;
      scheduleDraw();
    },
    /**
     * @param {Uint8Array} membership
     * @param {boolean} hidden
     */
    setFilterHidden(membership, hidden) {
      const change = hidden ? 1 : -1;

      hiddenFilterCount += change;
      for (let index = 0; index < hiddenCounts.length; index += 1) {
        if (membership[index]) hiddenCounts[index] += change;
      }

      scheduleDraw();
    },
  });
}

/** @typedef {import("../data.js").BlockPreviewData} BlockPreviewData */
/** @typedef {import("../data.js").BlockPreviewTransaction} BlockPreviewTransaction */

/**
 * @typedef {Object} BlockPreviewPointer
 * @property {number} clientX
 * @property {number} clientY
 */

/** @typedef {import("./geometry.js").PreviewGeometry} PreviewGeometry */
