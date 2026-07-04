#!/usr/bin/env node
/**
 * Apply Litecoin display labels to generated client cohort constants.
 * Mirrors crates/brk_bindgen/src/coin_labels.rs.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const REPLACEMENTS = [
  ["Bitcoin Research Kit", "Litecoin Research Kit"],
  ["Bitcoin Core", "Litecoin Core"],
  ["Bitcoin address", "Litecoin address"],
  ["Bitcoin block", "Litecoin block"],
  ["Bitcoin amount", "Litecoin amount"],
  ["Bitcoin protocol", "Litecoin protocol"],
  ["Bitcoin network", "Litecoin network"],
  ["Bitcoin node", "Litecoin node"],
  ["Bitcoin consensus", "Litecoin consensus"],
  ["Bitcoin data", "Litecoin data"],
  ["Bitcoin primitives", "Litecoin primitives"],
  ["bitcoin latest", "litecoin latest"],
  ["Bitview", "Litview"],
  ["BTC/USD", "LTC/USD"],
  [
    "satoshis (1 BTC = 100,000,000 sats)",
    "litoshis (1 LTC = 100,000,000 lits)",
  ],
  [
    "Fractional satoshis (f64) - for representing USD prices in sats",
    "Fractional litoshis (f64) - for representing USD prices in lits",
  ],
  ["When BTC is $100,000:", "When LTC is $100,000:"],
  ["supply in BTC", "supply in LTC"],
  ["in BTC.", "in LTC."],
  ["satoshis", "litoshis"],
  [" in sats", " in lits"],
  [" (sats)", " (lits)"],
  [" × sats", " × lits"],
  [" Sats", " Lits"],
  [" sats", " lits"],
  [" Sat", " Lit"],
  [" BTC", " LTC"],
  ["BTC-weighted", "LTC-weighted"],
];

const COHORT_CONSTANTS = [
  "AMOUNT_RANGE_NAMES",
  "OVER_AMOUNT_NAMES",
  "UNDER_AMOUNT_NAMES",
  "AGE_RANGE_NAMES",
  "UNDER_AGE_NAMES",
  "OVER_AGE_NAMES",
  "TERM_NAMES",
  "EPOCH_NAMES",
  "CLASS_NAMES",
  "ENTRY_NAMES",
  "SPENDABLE_TYPE_NAMES",
  "PROFITABILITY_RANGE_NAMES",
  "PROFIT_NAMES",
  "LOSS_NAMES",
];

/** @param {string} text */
function relabelDisplayText(text) {
  if (text === "0 sats") return "0 litoshis";
  if (text === "0 Sats") return "0 Litoshis";

  let out = text;
  for (const [from, to] of REPLACEMENTS) {
    if (out.includes(from)) out = out.replaceAll(from, to);
  }
  return out;
}

/**
 * @param {unknown} value
 * @returns {unknown}
 */
function relabelCohortConstants(value) {
  if (Array.isArray(value)) {
    return value.map(relabelCohortConstants);
  }
  if (value && typeof value === "object") {
    /** @type {Record<string, unknown>} */
    const out = {};
    for (const [key, val] of Object.entries(value)) {
      if ((key === "short" || key === "long") && typeof val === "string") {
        out[key] = relabelDisplayText(val);
      } else {
        out[key] = relabelCohortConstants(val);
      }
    }
    return out;
  }
  return value;
}

/**
 * @param {string} src
 * @param {string} constName
 */
function relabelJsConstantBlock(src, constName) {
  const marker = `${constName} = /** @type {const} */ (`;
  const start = src.indexOf(marker);
  if (start < 0) throw new Error(`Missing ${constName} in JS client`);

  let i = start + marker.length;
  let depth = 1;
  while (i < src.length && depth > 0) {
    const ch = src[i];
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
    i++;
  }

  const objectSrc = src.slice(start + marker.length, i - 1);
  const value = Function(`"use strict"; return (${objectSrc});`)();
  const relabeled = relabelCohortConstants(value);
  const formatted = JSON.stringify(relabeled, null, 2)
    .replaceAll('"\n', '"\n')
    .replace(/^/gm, "    ");

  return (
    src.slice(0, start) +
    `${constName} = /** @type {const} */ (${formatted}\n  );` +
    src.slice(i + 1)
  );
}

/**
 * @param {string} src
 * @param {string} constName
 */
function relabelPyConstantBlock(src, constName) {
  const marker = `${constName} = {`;
  const start = src.indexOf(marker);
  if (start < 0) throw new Error(`Missing ${constName} in Python client`);

  let i = start + marker.length;
  let depth = 1;
  while (i < src.length && depth > 0) {
    const ch = src[i];
    if (ch === "{") depth++;
    else if (ch === "}") depth--;
    i++;
  }

  const objectSrc = `{${src.slice(start + marker.length, i - 1)}}`;
  const value = JSON.parse(
    objectSrc
      .replaceAll("'", '"')
      .replace(/,\s*}/g, "}")
      .replace(/,\s*]/g, "]")
      .replace(/True/g, "true")
      .replace(/False/g, "false")
      .replace(/None/g, "null"),
  );
  const relabeled = relabelCohortConstants(value);

  const lines = ["{"];
  for (const [key, entry] of Object.entries(relabeled)) {
    lines.push(`        "${key}": {`);
    for (const [field, fieldVal] of Object.entries(
      /** @type {Record<string, string>} */ (entry),
    )) {
      lines.push(`            "${field}": ${JSON.stringify(fieldVal)},`);
    }
    lines[lines.length - 1] = lines[lines.length - 1].replace(/,$/, "");
    lines.push("        },");
  }
  lines[lines.length - 1] = lines[lines.length - 1].replace(/,$/, "");
  lines.push("    }");

  return (
    src.slice(0, start) + `${constName} = ${lines.join("\n")}` + src.slice(i)
  );
}

const jsPath = path.join(root, "modules/brk-client/index.js");
let js = fs.readFileSync(jsPath, "utf8");
for (const name of COHORT_CONSTANTS) {
  js = relabelJsConstantBlock(js, name);
}
fs.writeFileSync(jsPath, js);

const pyPath = path.join(root, "packages/brk_client/brk_client/__init__.py");
let py = fs.readFileSync(pyPath, "utf8");
for (const name of COHORT_CONSTANTS) {
  py = relabelPyConstantBlock(py, name);
}
fs.writeFileSync(pyPath, py);

console.log("Relabeled cohort constants in JS and Python clients");
