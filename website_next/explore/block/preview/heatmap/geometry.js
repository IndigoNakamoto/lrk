const GAP_REFERENCE_WIDTH = 640;

/**
 * @param {number} count
 * @param {number} cell
 * @param {number} gap
 */
function unitsToPixels(count, cell, gap) {
  return count * cell + Math.max(0, count - 1) * gap;
}

/**
 * @param {HTMLCanvasElement} canvas
 * @param {number} width
 */
function getGap(canvas, width) {
  const maxGap = Number.parseFloat(
    getComputedStyle(canvas).getPropertyValue("--block-preview-heatmap-gap"),
  ) || 0;

  return Math.max(1, maxGap * Math.min(1, width / GAP_REFERENCE_WIDTH));
}

/**
 * @param {Uint16Array} lookup
 * @param {number} columns
 * @param {SquareCellLayout} layout
 * @param {number} value
 */
function fillLookup(lookup, columns, layout, value) {
  const endX = layout.x + layout.span;
  const endY = layout.y + layout.span;

  for (let row = layout.y; row < endY; row += 1) {
    const offset = row * columns;

    for (let column = layout.x; column < endX; column += 1) {
      lookup[offset + column] = value;
    }
  }
}

/**
 * @param {SquareLayout} square
 * @param {HTMLCanvasElement} canvas
 * @param {number} width
 */
export function createPreviewGeometry(square, canvas, width) {
  const gap = getGap(canvas, width);
  const cell = Math.max(1, (width - gap * (square.columns - 1)) / square.columns);
  const unit = cell + gap;
  const lookup = new Uint16Array(square.columns * square.columns);

  const rects = square.layouts.map((layout, index) => {
    const transaction = square.resolvedCells[index].transaction;
    const rectSize = unitsToPixels(layout.span, cell, gap);

    fillLookup(lookup, square.columns, layout, index + 1);

    return {
      color: square.resolvedCells[index].color,
      size: rectSize,
      transaction,
      x: layout.x * unit,
      y: width - layout.y * unit - rectSize,
    };
  });

  return { columns: square.columns, lookup, rects, unit, width };
}

/**
 * @param {PreviewGeometry} geometry
 * @param {number} x
 * @param {number} y
 */
export function hitTest(geometry, x, y) {
  if (x < 0 || y < 0 || x > geometry.width || y > geometry.width) {
    return null;
  }

  const column = Math.min(geometry.columns - 1, Math.floor(x / geometry.unit));
  const rowFromTop = Math.min(
    geometry.columns - 1,
    Math.floor(y / geometry.unit),
  );
  const row = geometry.columns - rowFromTop - 1;
  const index = geometry.lookup[row * geometry.columns + column] - 1;

  if (index < 0) return null;

  const rect = geometry.rects[index];

  if (
    x >= rect.x &&
    x <= rect.x + rect.size &&
    y >= rect.y &&
    y <= rect.y + rect.size
  ) {
    return rect.transaction;
  }

  return null;
}

/**
 * @typedef {Object} PreviewRect
 * @property {string} color
 * @property {number} size
 * @property {BlockPreviewTransaction} transaction
 * @property {number} x
 * @property {number} y
 */

/**
 * @typedef {Object} PreviewGeometry
 * @property {number} columns
 * @property {Uint16Array} lookup
 * @property {PreviewRect[]} rects
 * @property {number} unit
 * @property {number} width
 */

/**
 * @typedef {Object} SquareLayout
 * @property {number} columns
 * @property {SquareCell[]} resolvedCells
 * @property {SquareCellLayout[]} layouts
 */

/**
 * @typedef {Object} SquareCell
 * @property {string} color
 * @property {BlockPreviewTransaction} transaction
 */

/**
 * @typedef {Object} SquareCellLayout
 * @property {number} span
 * @property {number} x
 * @property {number} y
 */

/** @typedef {import("../data.js").BlockPreviewTransaction} BlockPreviewTransaction */
