const CALIBRATION_STEPS = 24;

/**
 * @param {Float64Array} spans
 * @param {number} scale
 */
function scaledArea(spans, scale) {
  let area = 0;

  for (const span of spans) {
    const resolved = Math.max(1, Math.round(span * scale));

    area += resolved * resolved;
  }

  return area;
}

/**
 * @template {CapacityCell} Cell
 * @param {readonly Cell[]} cells
 * @param {number} blockWeight
 * @param {number} capacity
 * @param {number} columns
 */
export function resolveCapacityCells(cells, blockWeight, capacity, columns) {
  const gridArea = columns * columns;
  const targetArea = Math.max(
    cells.length,
    Math.floor((gridArea * blockWeight) / capacity),
  );
  const idealSpans = new Float64Array(cells.length);
  const resolved = /** @type {ResolvedCapacityCell<Cell>[]} */ (cells);

  for (let index = 0; index < cells.length; index += 1) {
    idealSpans[index] = Math.sqrt((cells[index].weight * gridArea) / capacity);
  }

  let lowerScale = 0;
  let upperScale = 1;

  while (scaledArea(idealSpans, upperScale) <= targetArea) {
    upperScale *= 2;
  }

  for (let step = 0; step < CALIBRATION_STEPS; step += 1) {
    const scale = (lowerScale + upperScale) / 2;

    if (scaledArea(idealSpans, scale) <= targetArea) {
      lowerScale = scale;
    } else {
      upperScale = scale;
    }
  }

  let area = 0;
  let largestSpan = 1;

  for (let index = 0; index < resolved.length; index += 1) {
    const idealSpan = idealSpans[index];
    const span = Math.max(1, Math.round(idealSpan * lowerScale));

    resolved[index].span = span;
    idealSpans[index] = (span + 0.5) / idealSpan;
    largestSpan = Math.max(largestSpan, span);
    area += span * span;
  }

  const candidates = Uint32Array.from(resolved, (_, index) => index);

  candidates.sort((a, b) => {
    return idealSpans[a] - idealSpans[b] || a - b;
  });

  for (const index of candidates) {
    const cell = resolved[index];
    const growth = 2 * cell.span + 1;

    if (area + growth > targetArea) continue;

    cell.span += 1;
    largestSpan = Math.max(largestSpan, cell.span);
    area += growth;
    if (targetArea - area < 3) break;
  }

  return {
    resolvedCells: resolved,
    rows: Math.max(largestSpan, Math.ceil(targetArea / columns)),
  };
}

/**
 * @typedef {Object} CapacityCell
 * @property {number} weight
 */

/**
 * @template {CapacityCell} Cell
 * @typedef {Cell & { span: number }} ResolvedCapacityCell
 */
