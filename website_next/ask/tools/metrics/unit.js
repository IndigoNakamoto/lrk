/** @param {string} type */
export function unitFromType(type) {
  const value = type.toLowerCase();
  if (value.includes("dollar") || value.includes("usd") || value.includes("cents")) return "usd";
  if (value.includes("bitcoin") || value === "btc") return "btc";
  if (value.includes("percent")) return "percent";
  if (value.includes("address")) return "addresses";
  if (value.includes("utxo") || value.includes("output")) return "utxos";
  if (value.includes("block") || value.includes("height")) return "blocks";
  return "number";
}
