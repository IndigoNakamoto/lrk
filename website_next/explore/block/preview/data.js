import { brk } from "../../../utils/client.js";
import {
  FILTERS,
  TYPE_BITS,
  TYPE_FILTER_MASK,
  getFilterBit,
} from "./filters/model.js";

const VERSION_BITS = [
  0,
  getFilterBit("version:1"),
  getFilterBit("version:2"),
  getFilterBit("version:3"),
];

const RBF_YES_BIT = getFilterBit("rbf:yes");
const RBF_NO_BIT = getFilterBit("rbf:no");
const INPUT_ONE_BIT = getFilterBit("input:one");
const INPUT_MULTI_BIT = getFilterBit("input:multi");
const OUTPUT_ONE_BIT = getFilterBit("output:one");
const OUTPUT_MULTI_BIT = getFilterBit("output:multi");
const FILTER_INDEX_BY_BIT = FILTERS.reduce((indexes, { bit, index }) => {
  indexes[bit] = index;

  return indexes;
}, /** @type {Record<number, number>} */ ({}));

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
    fetchSeriesSlice(tx.size.weight.txIndex.by.tx_index, start, end, signal),
    fetchSeriesSlice(tx.fees.feeRate.by.tx_index, start, end, signal),
  ]);

  signal.throwIfAborted();

  return {
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
 * @param {number[]} starts
 * @param {number[]} counts
 */
function rangeEnd(starts, counts) {
  const last = starts.length - 1;

  return starts[last] + counts[last];
}

/**
 * @param {Uint32Array} counts
 * @param {number} mask
 */
function countMask(counts, mask) {
  while (mask) {
    const bit = mask & -mask;

    counts[FILTER_INDEX_BY_BIT[bit]] += 1;
    mask ^= bit;
  }
}

/**
 * @param {readonly string[]} types
 * @param {number} start
 * @param {number} end
 */
function getTypesMask(types, start, end) {
  let mask = 0;

  for (let index = start; index < end; index += 1) {
    mask |= TYPE_BITS[/** @type {keyof typeof TYPE_BITS} */ (types[index])] ?? 0;
    if (mask === TYPE_FILTER_MASK) return mask;
  }

  return mask;
}

/**
 * @param {BlockPreviewRange} range
 * @param {AbortSignal} signal
 */
export async function loadBlockPreviewFilters(range, signal) {
  const { start, end } = range;
  const tx = brk.series.transactions;
  const indexes = brk.series.indexes.txIndex;
  const [
    versions,
    rbfs,
    inputCounts,
    outputCounts,
    firstInputs,
    firstOutputs,
  ] = await Promise.all([
    fetchSeriesSlice(tx.raw.txVersion.by.tx_index, start, end, signal),
    fetchSeriesSlice(tx.raw.isExplicitlyRbf.by.tx_index, start, end, signal),
    fetchSeriesSlice(indexes.inputCount.by.tx_index, start, end, signal),
    fetchSeriesSlice(indexes.outputCount.by.tx_index, start, end, signal),
    fetchSeriesSlice(tx.raw.firstTxinIndex.by.tx_index, start, end, signal),
    fetchSeriesSlice(tx.raw.firstTxoutIndex.by.tx_index, start, end, signal),
  ]);

  signal.throwIfAborted();

  const inputStart = /** @type {number} */ (firstInputs.data[0]);
  const outputStart = /** @type {number} */ (firstOutputs.data[0]);
  const [inputTypes, outputTypes] = await Promise.all([
    fetchSeriesSlice(
      brk.series.inputs.raw.outputType.by.txin_index,
      inputStart,
      rangeEnd(
        /** @type {number[]} */ (firstInputs.data),
        /** @type {number[]} */ (inputCounts.data),
      ),
      signal,
    ),
    fetchSeriesSlice(
      brk.series.outputs.raw.outputType.by.txout_index,
      outputStart,
      rangeEnd(
        /** @type {number[]} */ (firstOutputs.data),
        /** @type {number[]} */ (outputCounts.data),
      ),
      signal,
    ),
  ]);

  signal.throwIfAborted();

  const masks = new Uint32Array(end - start);
  const counts = new Uint32Array(FILTERS.length);

  for (let index = 0; index < versions.data.length; index += 1) {
    const version = /** @type {number} */ (versions.data[index]);
    const inputCount = /** @type {number} */ (inputCounts.data[index]);
    const outputCount = /** @type {number} */ (outputCounts.data[index]);
    const inputTypeStart = /** @type {number} */ (firstInputs.data[index]);
    const inputTypeEnd = inputTypeStart + inputCount;
    const outputTypeStart = /** @type {number} */ (firstOutputs.data[index]);
    const outputTypeEnd = outputTypeStart + outputCount;
    const mask =
      (VERSION_BITS[version] ?? 0) |
      (rbfs.data[index] ? RBF_YES_BIT : RBF_NO_BIT) |
      (inputCount === 1 ? INPUT_ONE_BIT : INPUT_MULTI_BIT) |
      (outputCount === 1 ? OUTPUT_ONE_BIT : OUTPUT_MULTI_BIT) |
      getTypesMask(
        /** @type {string[]} */ (inputTypes.data),
        inputTypeStart - inputStart,
        inputTypeEnd - inputStart,
      ) |
      getTypesMask(
        /** @type {string[]} */ (outputTypes.data),
        outputTypeStart - outputStart,
        outputTypeEnd - outputStart,
      );

    masks[index] = mask;
    countMask(counts, mask);
  }

  return { counts, masks, start };
}

/**
 * @typedef {Object} BlockPreviewRange
 * @property {number} start
 * @property {number} end
 */

/**
 * @typedef {Object} BlockPreviewData
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

/**
 * @typedef {Object} BlockPreviewFilterState
 * @property {number} start
 * @property {Uint32Array} masks
 * @property {Uint32Array} counts
 */
