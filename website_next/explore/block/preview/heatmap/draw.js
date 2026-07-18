import { getCanvasColor } from "./color.js";

const HOVER_FILL_ALPHA = 0.18;
const HOVER_MARKER_MIN_SIZE = 10;
const MUTED_ALPHA = 0.12;

const CANVAS_COLORS = {
  black: getCanvasColor("var(--black)"),
  white: getCanvasColor("var(--white)"),
};

/**
 * @param {CanvasRenderingContext2D} context
 * @param {number} alpha
 * @param {string} color
 * @param {PreviewRect} rect
 */
function drawRect(context, alpha, color, rect) {
  context.globalAlpha = alpha;
  context.fillStyle = color;
  context.fillRect(rect.x, rect.y, rect.size, rect.size);
}

/**
 * @param {CanvasRenderingContext2D} context
 * @param {PreviewRect} rect
 */
function drawHover(context, rect) {
  const size = Math.max(rect.size, HOVER_MARKER_MIN_SIZE);
  const x = rect.x + rect.size / 2 - size / 2;
  const y = rect.y + rect.size / 2 - size / 2;
  const width = Math.max(1, Math.min(3, size / 5));

  context.fillStyle = CANVAS_COLORS.white;
  context.globalAlpha = HOVER_FILL_ALPHA;
  context.fillRect(x, y, size, size);
  context.globalAlpha = 1;
  context.lineJoin = "miter";
  context.lineWidth = width + 2;
  context.strokeStyle = CANVAS_COLORS.black;
  context.strokeRect(x, y, size, size);
  context.lineWidth = width;
  context.strokeStyle = CANVAS_COLORS.white;
  context.strokeRect(x, y, size, size);
}

/**
 * @param {DrawPreviewArgs} args
 */
export function drawPreview(args) {
  const {
    context,
    hasHidden,
    hiddenCounts,
    inspected,
    previewMembership,
    rects,
    start,
  } = args;
  let inspectedRect = /** @type {PreviewRect | null} */ (null);

  if (previewMembership === null && !hasHidden) {
    for (const rect of rects) {
      drawRect(context, 1, rect.color, rect);
      if (rect.transaction === inspected) inspectedRect = rect;
    }
  } else {
    for (const rect of rects) {
      const index = rect.transaction.txIndex - start;
      const alpha = previewMembership === null
        ? (hiddenCounts[index] ? MUTED_ALPHA : 1)
        : (previewMembership[index] ? 1 : MUTED_ALPHA);

      drawRect(context, alpha, rect.color, rect);
      if (rect.transaction === inspected) inspectedRect = rect;
    }
  }

  if (inspectedRect !== null) drawHover(context, inspectedRect);
  context.globalAlpha = 1;
}

/**
 * @typedef {Object} DrawPreviewArgs
 * @property {CanvasRenderingContext2D} context
 * @property {boolean} hasHidden
 * @property {Uint8Array} hiddenCounts
 * @property {BlockPreviewTransaction | null} inspected
 * @property {Uint8Array | null} previewMembership
 * @property {PreviewRect[]} rects
 * @property {number} start
 */

/** @typedef {import("../data.js").BlockPreviewTransaction} BlockPreviewTransaction */
/** @typedef {import("./geometry.js").PreviewRect} PreviewRect */
