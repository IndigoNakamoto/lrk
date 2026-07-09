export const DIFFICULTY_EPOCH_BLOCKS = 2_016;
export const HALVING_EPOCH_BLOCKS = 210_000;

const MAX_BLOCK_WEIGHT = 4_000_000;

/** @param {number} value */
export function formatNumber(value) {
  return value.toLocaleString();
}

/** @param {number | undefined} value */
export function formatPoolBlockNumber(value) {
  // Temporary compatibility with servers that do not include pool.blockNumber yet.
  return `#${formatNumber(value ?? 0)}`;
}

/** @param {number} unixSeconds */
export function formatDateTime(unixSeconds) {
  return new Date(unixSeconds * 1_000).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "medium",
  });
}

/** @param {number} bytes */
export function formatBytes(bytes) {
  return bytes >= 1_000_000
    ? `${(bytes / 1_000_000).toFixed(2)} MB`
    : `${formatNumber(bytes)} B`;
}

/** @param {number} weight */
export function formatWeight(weight) {
  return weight >= 1_000_000
    ? `${(weight / 1_000_000).toFixed(2)} MWU`
    : `${formatNumber(weight)} WU`;
}

/** @param {number} weight */
export function formatBlockFill(weight) {
  return `${((weight / MAX_BLOCK_WEIGHT) * 100).toFixed(1)}%`;
}

/** @param {number} height @param {number} length */
export function getEpochProgress(height, length) {
  const blocks = (height % length) + 1;

  return /** @type {const} */ ({
    number: Math.floor(height / length) + 1,
    blocks,
    remaining: length - blocks,
    ratio: blocks / length,
  });
}

/** @param {number} height @param {number} length */
export function formatEpoch(height, length) {
  const progress = getEpochProgress(height, length);

  return `#${formatNumber(progress.number)} · ${(progress.ratio * 100).toFixed(1)}%`;
}

/** @param {string} raw */
export function getCoinbaseMessage(raw) {
  return (raw.match(/[\x20-\x7e]{2,}/g) ?? [])
    .map((value) => value.trim())
    .filter((value) => /[A-Za-z0-9]/.test(value))
    .join(" · ");
}
