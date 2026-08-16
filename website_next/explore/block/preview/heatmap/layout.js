import { resolveCapacityCells } from "./capacity.js";
import { packTransactions } from "./fees.js";

/**
 * @template {{ weight: number }} Cell
 * @param {readonly Cell[]} cells
 * @param {number} blockWeight
 * @param {number} capacity
 * @param {number} columns
 */
export function createSquareLayout(cells, blockWeight, capacity, columns) {
  const capacityLayout = resolveCapacityCells(
    cells,
    blockWeight,
    capacity,
    columns,
  );
  const packed = packTransactions(
    capacityLayout.resolvedCells,
    columns,
    capacityLayout.rows,
  );

  return { columns, ...packed };
}
