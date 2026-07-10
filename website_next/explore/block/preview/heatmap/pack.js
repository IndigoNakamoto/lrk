/**
 * @param {readonly PackCell[]} cells
 * @param {number} columns
 * @param {number} rows
 * @returns {PackLayout[] | null}
 */
export function packCells(cells, columns, rows) {
  const occupied = new Uint8Array(columns * rows);
  const layouts = [];

  for (const cell of cells) {
    const span = Math.min(cell.span, columns);
    const position = findPosition(occupied, columns, rows, span);

    if (position === null) return null;

    fillCells(occupied, columns, position.x, position.y, span);
    layouts.push({ x: position.x, y: position.y, span });
  }

  return layouts;
}

/**
 * @param {Uint8Array} occupied
 * @param {number} columns
 * @param {number} rows
 * @param {number} span
 * @returns {{ x: number, y: number } | null}
 */
function findPosition(occupied, columns, rows, span) {
  const lastRow = rows - span;

  for (let y = 0; y <= lastRow; y += 1) {
    for (let x = 0; x <= columns - span; x += 1) {
      if (canPlace(occupied, columns, x, y, span)) return { x, y };
    }
  }

  return null;
}

/**
 * @param {Uint8Array} occupied
 * @param {number} columns
 * @param {number} x
 * @param {number} y
 * @param {number} span
 */
function canPlace(occupied, columns, x, y, span) {
  for (let row = y; row < y + span; row += 1) {
    const offset = row * columns;

    for (let column = x; column < x + span; column += 1) {
      if (occupied[offset + column]) return false;
    }
  }

  return true;
}

/**
 * @param {Uint8Array} occupied
 * @param {number} columns
 * @param {number} x
 * @param {number} y
 * @param {number} span
 */
function fillCells(occupied, columns, x, y, span) {
  for (let row = y; row < y + span; row += 1) {
    const offset = row * columns;

    for (let column = x; column < x + span; column += 1) {
      occupied[offset + column] = 1;
    }
  }
}

/**
 * @typedef {Object} PackCell
 * @property {number} span
 */

/**
 * @typedef {Object} PackLayout
 * @property {number} x
 * @property {number} y
 * @property {number} span
 */
