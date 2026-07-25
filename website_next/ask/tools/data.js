import { brk } from "../../utils/client.js";

const RANGE_POINTS = 120;

/** @param {string} type */
export function unitFromType(type) {
  const value = type.toLowerCase();
  if (value.includes("dollar") || value.includes("usd") || value.includes("cents")) return "usd";
  if (value.includes("bitcoin") || value === "btc") return "btc";
  if (value.includes("percent") || value.includes("ratio")) return "percent";
  if (value.includes("address")) return "addresses";
  if (value.includes("utxo") || value.includes("output")) return "utxos";
  if (value.includes("block") || value.includes("height")) return "blocks";
  return undefined;
}

/** @param {string} unit */
function displayedUnit(unit) {
  if (unit === "usd") return "$";
  if (unit === "percent") return "%";
  if (unit === "btc") return " BTC";
  if (unit === "addresses") return " addresses";
  if (unit === "utxos") return " UTXOs";
  if (unit === "blocks") return " blocks";
  return "";
}

/** @param {number} value @param {string | undefined} unit */
export function formatValue(value, unit) {
  const number = new Intl.NumberFormat("en-US", { maximumFractionDigits: 8 }).format(value);
  const affix = displayedUnit(unit ?? "");
  return affix === "$" ? `$${number}` : `${number}${affix}`;
}

/** @param {{ name: string, indexes: string[], type: string, suggestedUnit?: string }} metric @param {Record<string, unknown>} action */
export async function readMetric(metric, action) {
  const rawIndex = typeof action.index === "string" ? action.index : "";
  const indexLooksLikeValue = /^-?\d+$/.test(rawIndex) || /^\d{4}-\d{2}-\d{2}$/.test(rawIndex);
  const at = action.at ?? (indexLooksLikeValue ? rawIndex : undefined);
  const dateLike = typeof at === "string" && !/^-?\d+$/.test(at);
  const preferredIndex = indexLooksLikeValue ? "" : rawIndex;
  const index = metric.indexes.includes(preferredIndex)
    ? preferredIndex
    : dateLike && metric.indexes.includes("day1")
      ? "day1"
      : metric.indexes.includes("height")
        ? "height"
        : metric.indexes[0];
  if (!index) throw new Error(`No supported index for ${metric.name}`);
  const mode = typeof action.mode === "string" ? action.mode : "latest";
  let response;

  if (mode === "at") {
    if (at === undefined) throw new Error("A block height or date is required");
    response = await brk.getSeries(
      metric.name,
      /** @type {any} */ (index),
      /** @type {any} */ (at),
      undefined,
      1,
    );
  } else if (mode === "range") {
    const points = Math.min(Number(action.points) || RANGE_POINTS, RANGE_POINTS);
    response = await brk.getSeries(
      metric.name,
      /** @type {any} */ (index),
      /** @type {any} */ (action.start ?? -points),
      /** @type {any} */ (action.end),
      points,
    );
  } else {
    response = await brk.getSeries(
      metric.name,
      /** @type {any} */ (index),
      -1,
      undefined,
      1,
    );
  }

  if (!response || typeof response !== "object" || !Array.isArray(response.data)) {
    throw new Error(`No data returned for ${metric.name}`);
  }
  return {
    name: metric.name,
    label: metric.name.replaceAll("_", " "),
    unit: metric.suggestedUnit ?? unitFromType(metric.type),
    index: response.index,
    start: response.start,
    end: response.end,
    stamp: response.stamp,
    values: response.data,
  };
}
