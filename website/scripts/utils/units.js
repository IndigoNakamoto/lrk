/** Unit definitions for chart series */

/**
 * Unit enum with id (for serialization) and name (for display)
 */
export const Unit = /** @type {const} */ ({
  // Value units
  sats: { id: "sats", name: "Litoshis" },
  // Coin unit — name matches the active chain (Bitcoin or Litecoin).
  // Set via setCoinUnitName() once chain info is available from the server.
  btc: { id: "btc", name: "Litecoin" },
  usd: { id: "usd", name: "US Dollars" },

  // Ratios & percentages
  percentage: { id: "percentage", name: "Percentage" },
  cagr: { id: "cagr", name: "CAGR (%/year)" },
  ratio: { id: "ratio", name: "Ratio" },
  index: { id: "index", name: "Index" },
  sd: { id: "sd", name: "Std Dev" },

  // Relative percentages
  pctSupply: { id: "pct-supply", name: "% of circulating" },
  pctOwn: { id: "pct-own", name: "% of Own" },

  // Time
  days: { id: "days", name: "Days" },
  years: { id: "years", name: "Years" },
  secs: { id: "secs", name: "Seconds" },

  // Counts
  count: { id: "count", name: "Count" },
  blocks: { id: "blocks", name: "Blocks" },

  // Size
  bytes: { id: "bytes", name: "Bytes" },
  vb: { id: "vb", name: "Virtual Bytes" },
  wu: { id: "wu", name: "Weight Units" },

  // Mining
  hashRate: { id: "hashrate", name: "Hash Rate" },
  difficulty: { id: "difficulty", name: "Difficulty" },
  epoch: { id: "epoch", name: "Epoch" },

  // Fees
  feeRate: { id: "feerate", name: "Lit/vByte" },

  // Rates
  perSec: { id: "per-sec", name: "Per Second" },

  // Cointime
  coinblocks: { id: "coinblocks", name: "Coinblocks" },
  coindays: { id: "coindays", name: "Coindays" },
  satblocks: { id: "satblocks", name: "Litblocks" },
  satdays: { id: "satdays", name: "Litdays" },

  // Hash price/value
  usdPerThsPerDay: { id: "usd-ths-day", name: "USD/TH/s/Day" },
  usdPerPhsPerDay: { id: "usd-phs-day", name: "USD/PH/s/Day" },
  satsPerThsPerDay: { id: "sats-ths-day", name: "Lits/TH/s/Day" },
  satsPerPhsPerDay: { id: "sats-phs-day", name: "Lits/PH/s/Day" },
});

/** @typedef {keyof typeof Unit} UnitKey */
/** @typedef {typeof Unit[UnitKey]} UnitObject */

/**
 * Update the coin unit display name once chain info is known.
 * Call this after fetching chain metadata from the server (e.g., the health endpoint).
 * @param {string} coinName - e.g. "Bitcoin" or "Litecoin"
 */
export function setCoinUnitName(coinName) {
    Unit.btc.name = coinName;
}
