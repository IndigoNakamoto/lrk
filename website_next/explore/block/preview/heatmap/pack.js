/**
 * @param {readonly PackCell[]} cells
 * @param {number} columns
 * @param {number} rows
 * @returns {PackLayout[]}
 */
export function packCells(cells, columns, rows) {
  const occupied = new Uint8Array(columns * rows);
  const cursors = new Uint32Array(columns + 1);
  const layouts = [];

  for (const cell of cells) {
    let span = Math.min(cell.span, columns);
    let position = findPosition(
      occupied,
      columns,
      rows,
      span,
      cursors[span],
    );

    while (position === null) {
      span -= 1;
      position = findPosition(
        occupied,
        columns,
        rows,
        span,
        cursors[span],
      );
    }

    fillCells(occupied, columns, position.x, position.y, span);
    cursors[span] = position.index + span;
    layouts.push({ x: position.x, y: position.y, span });
  }

  return layouts;
}

/**
 * @param {Uint8Array} occupied
 * @param {number} columns
 * @param {number} rows
 * @param {number} span
 * @param {number} start
 * @returns {{ index: number, x: number, y: number } | null}
 */
function findPosition(occupied, columns, rows, span, start) {
  const lastRow = rows - span;
  const lastColumn = columns - span;
  const startRow = Math.floor(start / columns);
  const startColumn = start % columns;

  for (let y = startRow; y <= lastRow; y += 1) {
    const firstColumn = y === startRow ? startColumn : 0;

    for (let x = firstColumn; x <= lastColumn; x += 1) {
      if (canPlace(occupied, columns, x, y, span)) {
        return { index: y * columns + x, x, y };
      }
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
