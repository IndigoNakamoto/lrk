import { FEE_RATE_PERCENTILES } from "../../fee-rates.js";
import { packCells } from "./pack.js";

/**
 * @param {number[]} feeRates
 * @param {Uint32Array} order
 */
export function createPreviewFeeRange(feeRates, order) {
  return FEE_RATE_PERCENTILES.map((percentile) => {
    const index = Math.round(((100 - percentile) / 100) * (order.length - 1));

    return feeRates[order[index]];
  });
}

/**
 * @param {number[]} weights
 * @param {number[]} feeRates
 */
export function orderTransactions(weights, feeRates) {
  const order = new Uint32Array(feeRates.length);

  for (let index = 0; index < order.length; index += 1) {
    order[index] = index;
  }

  return order.sort((a, b) => feeRates[b] - feeRates[a] || weights[b] - weights[a]);
}

/**
 * @template {{ span: number }} Cell
 * @param {readonly Cell[]} cells
 * @param {number} columns
 * @param {number} rows
 */
export function packTransactions(cells, columns, rows) {
  let layouts = packCells(cells, columns, rows);

  while (layouts === null) {
    rows += 1;
    layouts = packCells(cells, columns, rows);
  }

  return { layouts, resolvedCells: cells };
}
