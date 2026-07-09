export { formatUsd } from "../usd/index.js";

/**
 * @param {number} value
 */
export function formatNumber(value) {
  return new Intl.NumberFormat("en-US").format(value);
}
