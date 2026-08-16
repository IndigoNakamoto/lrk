import { brk } from "../../../utils/client.js";

/**
 * @template T
 * @param {{
 *   slice: (
 *     start: number,
 *     end: number,
 *   ) => {
 *     fetch: (
 *       options: { signal: AbortSignal, memCache: false },
 *     ) => Promise<{ data: T[] }>,
 *   },
 * }} series
 * @param {number} start
 * @param {number} end
 * @param {AbortSignal} signal
 * @returns {Promise<{ data: T[] }>}
 */
function fetchSeriesSlice(series, start, end, signal) {
  return series.slice(start, end).fetch({ signal, memCache: false });
}

/**
 * @param {import("../../../modules/brk-client/index.js").BlockInfoV1} block
 * @param {AbortSignal} signal
 */
async function loadBlockPreviewRange(block, signal) {
  const firstTxIndex = (
    await fetchSeriesSlice(
      brk.series.transactions.raw.firstTxIndex.by.height,
      block.height,
      block.height + 1,
      signal,
    )
  ).data[0];

  signal.throwIfAborted();

  const start = /** @type {number} */ (firstTxIndex);
  const end = start + block.txCount;

  return { start, end };
}

/**
 * @param {import("../../../modules/brk-client/index.js").BlockInfoV1} block
 * @param {AbortSignal} signal
 */
export async function loadBlockPreview(block, signal) {
  const { start, end } = await loadBlockPreviewRange(block, signal);
  const tx = brk.series.transactions;
  const [weights, feeRates] = await Promise.all([
    fetchSeriesSlice(tx.raw.weight.by.tx_index, start, end, signal),
    fetchSeriesSlice(tx.fees.feeRate.by.tx_index, start, end, signal),
  ]);

  signal.throwIfAborted();

  return {
    blockWeight: block.weight,
    range: { start, end },
    feeRates: /** @type {number[]} */ (feeRates.data),
    weights: /** @type {number[]} */ (weights.data),
  };
}

/**
 * @param {number} txIndex
 * @param {AbortSignal} signal
 */
export async function loadBlockPreviewTxid(txIndex, signal) {
  const txid = (
    await brk.series.transactions.raw.txid.by.tx_index
      .get(txIndex)
      .fetch({ signal, memCache: false })
  ).data[0];

  signal.throwIfAborted();

  return /** @type {string} */ (txid);
}

/**
 * @typedef {Object} BlockPreviewRange
 * @property {number} start
 * @property {number} end
 */

/**
 * @typedef {Object} BlockPreviewData
 * @property {number} blockWeight
 * @property {BlockPreviewRange} range
 * @property {number[]} weights
 * @property {number[]} feeRates
 */

/**
 * @typedef {Object} BlockPreviewTransaction
 * @property {number} txIndex
 * @property {number} weight
 * @property {number} feeRate
 */
