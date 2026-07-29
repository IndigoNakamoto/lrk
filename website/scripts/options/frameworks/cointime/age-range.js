import { Unit } from "../../../utils/units.js";
import { line, ROLLING_WINDOWS } from "../../series.js";
import { satsBtcUsd } from "../../shared.js";

/**
 * @typedef {{
 *   average: Record<"_24h" | "_1w" | "_1m" | "_1y", AnySeriesPattern>,
 *   sum: Record<"_24h" | "_1w" | "_1m" | "_1y", AnySeriesPattern>,
 *   cumulative: AnySeriesPattern,
 * }} CointimeAgeRangeCoindays
 *
 * @typedef {Object} CointimeAgeRange
 * @property {string} name
 * @property {Color} color
 * @property {{
 *   coindaysCreated: CointimeAgeRangeCoindays,
 *   coindaysConsumed: CointimeAgeRangeCoindays,
 *   coindaysStored: CointimeAgeRangeCoindays,
 *   liveliness: AnySeriesPattern,
 *   vaultedness: AnySeriesPattern,
 *   ratio: AnySeriesPattern,
 *   supply: { active: AnyValuePattern, vaulted: AnyValuePattern },
 * }} tree
 */

/**
 * @param {readonly CointimeAgeRange[]} ranges
 * @param {"liveliness" | "vaultedness" | "ratio"} key
 * @param {string} name
 * @param {string} legend
 * @returns {PartialChartOption}
 */
function activityChart(ranges, key, name, legend) {
  return {
    name,
    title: `${legend} by UTXO Age`,
    bottom: ranges.map((range) =>
      line({
        series: range.tree[key],
        name: range.name,
        color: range.color,
        unit: Unit.ratio,
      }),
    ),
  };
}

/**
 * @param {readonly CointimeAgeRange[]} ranges
 * @param {"active" | "vaulted"} key
 * @param {string} name
 * @returns {PartialChartOption}
 */
function supplyChart(ranges, key, name) {
  return {
    name,
    title: `${name} Supply by UTXO Age`,
    bottom: ranges.flatMap((range) =>
      satsBtcUsd({
        pattern: range.tree.supply[key],
        name: range.name,
        color: range.color,
      }),
    ),
  };
}

/**
 * @param {readonly CointimeAgeRange[]} ranges
 * @param {"coindaysCreated" | "coindaysConsumed" | "coindaysStored"} key
 * @param {string} name
 * @returns {PartialOptionsGroup}
 */
function coindaysTree(ranges, key, name) {
  return {
    name,
    tree: [
      {
        name: "Average",
        tree: ROLLING_WINDOWS.map((window) => ({
          name: window.name,
          title: `${window.title} Average ${name} by UTXO Age`,
          bottom: ranges.map((range) =>
            line({
              series: range.tree[key].average[window.key],
              name: range.name,
              color: range.color,
              unit: Unit.coindays,
            }),
          ),
        })),
      },
      {
        name: "Sum",
        tree: ROLLING_WINDOWS.map((window) => ({
          name: window.name,
          title: `${window.title} ${name} by UTXO Age`,
          bottom: ranges.map((range) =>
            line({
              series: range.tree[key].sum[window.key],
              name: range.name,
              color: range.color,
              unit: Unit.coindays,
            }),
          ),
        })),
      },
      {
        name: "Cumulative",
        title: `Cumulative ${name} by UTXO Age`,
        bottom: ranges.map((range) =>
          line({
            series: range.tree[key].cumulative,
            name: range.name,
            color: range.color,
            unit: Unit.coindays,
          }),
        ),
      },
    ],
  };
}

/**
 * @param {readonly CointimeAgeRange[]} ranges
 * @returns {PartialOptionsGroup}
 */
export function createCointimeAgeRangeSection(ranges) {
  return {
    name: "Age Range",
    tree: [
      {
        name: "Supply",
        tree: [
          supplyChart(ranges, "active", "Active"),
          supplyChart(ranges, "vaulted", "Vaulted"),
        ],
      },
      {
        name: "Activity",
        tree: [
          activityChart(ranges, "liveliness", "Liveliness", "Liveliness"),
          activityChart(ranges, "vaultedness", "Vaultedness", "Vaultedness"),
          activityChart(
            ranges,
            "ratio",
            "Activity Ratio",
            "Liveliness / Vaultedness",
          ),
        ],
      },
      {
        name: "Coindays",
        tree: [
          coindaysTree(ranges, "coindaysCreated", "Coindays Created"),
          coindaysTree(ranges, "coindaysConsumed", "Coindays Consumed"),
          coindaysTree(ranges, "coindaysStored", "Coindays Stored"),
        ],
      },
    ],
  };
}
