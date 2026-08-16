export const DIFFICULTY_EPOCH_BLOCKS = 2_016;
export const HALVING_EPOCH_BLOCKS = 210_000;

export const MAX_BLOCK_WEIGHT = 4_000_000;

const RELATIVE_TIME = new Intl.RelativeTimeFormat(undefined);
const RELATIVE_UNITS = /** @type {const} */ ([
  ["year", 31_557_600],
  ["month", 2_629_800],
  ["day", 86_400],
  ["hour", 3_600],
  ["minute", 60],
]);

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

/** @param {number} unixSeconds */
export function formatDateAndAge(unixSeconds) {
  const date = new Date(unixSeconds * 1_000).toLocaleDateString(undefined, {
    dateStyle: "medium",
  });
  const difference = unixSeconds - Date.now() / 1_000;
  const absolute = Math.abs(difference);

  if (absolute < 60) return `${date} · just now`;

  const [unit, seconds] = RELATIVE_UNITS.find(([, duration]) => {
    return absolute >= duration;
  }) ?? RELATIVE_UNITS[RELATIVE_UNITS.length - 1];

  return `${date} · ${RELATIVE_TIME.format(Math.trunc(difference / seconds), unit)}`;
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
