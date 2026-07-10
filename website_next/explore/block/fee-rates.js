export const FEE_RATE_PERCENTILES = /** @type {const} */ ([
  0,
  10,
  25,
  50,
  75,
  90,
  100,
]);

export const FEE_RATE_STOPS = /** @type {const} */ ([
  { label: "min", color: "var(--emerald)" },
  { label: "10%", color: "var(--green)" },
  { label: "25%", color: "var(--lime)" },
  { label: "50%", color: "var(--yellow)" },
  { label: "75%", color: "var(--amber)" },
  { label: "90%", color: "var(--orange)" },
  { label: "max", color: "var(--red)" },
]);

/**
 * @param {number} feeRate
 * @param {number[]} ranges
 */
export function getFeeRateColor(feeRate, ranges) {
  if (feeRate <= ranges[0]) return FEE_RATE_STOPS[0].color;

  for (let index = 1; index < ranges.length; index += 1) {
    if (feeRate > ranges[index]) continue;

    const previousRate = ranges[index - 1];
    const nextRate = ranges[index];
    const span = nextRate - previousRate;
    const ratio = span ? (feeRate - previousRate) / span : 0;
    const previousShare = ((1 - ratio) * 100).toFixed(2);
    const nextShare = (ratio * 100).toFixed(2);

    return `color-mix(in oklch, ${FEE_RATE_STOPS[index - 1].color} ${previousShare}%, ${FEE_RATE_STOPS[index].color} ${nextShare}%)`;
  }

  return FEE_RATE_STOPS[FEE_RATE_STOPS.length - 1].color;
}
