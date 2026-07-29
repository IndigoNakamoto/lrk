import { brk } from "../../utils/client.js";
import { colors } from "../../utils/colors.js";
import { Unit } from "../../utils/units.js";
import { ageRanges } from "../age-ranges.js";
import { line, price } from "../series.js";
import { satsBtcUsd } from "../shared.js";

/**
 * @typedef {Object} CoinflowAgeRange
 * @property {string} name
 * @property {Color} color
 * @property {{
 *   mobility: AnySeriesPattern,
 *   spendingRate: AnySeriesPattern,
 *   spendingExposure: AnySeriesPattern,
 *   supply: { mobile: AnyValuePattern, immobile: AnyValuePattern },
 * }} tree
 */

/**
 * @param {readonly CoinflowAgeRange[]} ranges
 * @param {"mobility" | "spendingRate" | "spendingExposure"} key
 * @param {string} name
 * @returns {PartialChartOption}
 */
function ageRangeRatioChart(ranges, key, name) {
  return {
    name,
    title: `${name} by UTXO Age`,
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
 * @param {readonly CoinflowAgeRange[]} ranges
 * @param {"mobile" | "immobile"} key
 * @param {string} name
 * @returns {PartialChartOption}
 */
function ageRangeSupplyChart(ranges, key, name) {
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
 * Create Coinflow section.
 * @returns {PartialOptionsGroup}
 */
export function createCoinflowSection() {
  const { coinflow } = brk.series;
  const ranges = ageRanges.map(({ key, ...range }) => ({
    ...range,
    tree: coinflow.ageRange[key],
  }));

  const horizons = /** @type {const} */ ([
    { key: "_8y", name: "8Y" },
    { key: "_4y", name: "4Y" },
    { key: "_2y", name: "2Y" },
    { key: "_1y", name: "1Y" },
    { key: "_6m", name: "6M" },
    { key: "_3m", name: "3M" },
    { key: "_1m", name: "1M" },
  ]).map((horizon, index, all) => ({
    ...horizon,
    color: colors.at(index, all.length),
  }));

  return {
    name: "Coinflow",
    tree: [
      {
        name: "Supply",
        tree: [
          {
            name: "Breakdown",
            title: "Mobile vs Immobile Supply",
            bottom: [
              ...satsBtcUsd({
                pattern: coinflow.supply.mobile,
                name: "Mobile",
                color: colors.mobile,
              }),
              ...satsBtcUsd({
                pattern: coinflow.supply.immobile,
                name: "Immobile",
                color: colors.immobile,
              }),
            ],
          },
          {
            name: "Mobile in Loss",
            title: "Mobile Supply in Loss",
            bottom: [
              line({
                series: coinflow.supply.mobile.inLoss.share,
                name: "Lifetime",
                color: colors.loss,
                unit: Unit.ratio,
              }),
            ],
          },
          {
            name: "In Loss by Horizon",
            title: "Mobile Supply in Loss by Horizon",
            bottom: horizons.map((horizon) =>
              line({
                series:
                  coinflow.horizon[horizon.key].supply.inLoss.share,
                name: horizon.name,
                color: horizon.color,
                unit: Unit.ratio,
              }),
            ),
          },
        ],
      },
      {
        name: "Cap",
        title: "Coinflow Cap",
        bottom: [
          line({
            series: coinflow.cap.usd,
            name: "Coinflow",
            color: colors.coinflow,
            unit: Unit.usd,
          }),
        ],
      },
      {
        name: "Price",
        title: "Coinflow Price",
        top: [
          price({
            series: coinflow.price,
            name: "Coinflow",
            color: colors.coinflow,
          }),
        ],
        bottom: [
          line({
            series: coinflow.price.ratio,
            name: "Spot / Coinflow",
            color: colors.coinflow,
            unit: Unit.ratio,
          }),
        ],
      },
      {
        name: "Age Range",
        tree: [
          ageRangeRatioChart(ranges, "mobility", "Mobility"),
          ageRangeRatioChart(ranges, "spendingRate", "Spending Rate"),
          ageRangeRatioChart(
            ranges,
            "spendingExposure",
            "Spending Exposure",
          ),
          {
            name: "Supply",
            tree: [
              ageRangeSupplyChart(ranges, "mobile", "Mobile"),
              ageRangeSupplyChart(ranges, "immobile", "Immobile"),
            ],
          },
        ],
      },
    ],
  };
}
