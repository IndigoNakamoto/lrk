import { brk } from "../../utils/client.js";
import { colors } from "../../utils/colors.js";
import { Unit } from "../../utils/units.js";
import { baseline, histogram } from "../series.js";
import {
  percentileBands,
  priceBands,
  priceRatioPercentilesTree,
} from "../shared.js";

/**
 * Create Rarity Meter model section.
 * @returns {PartialOptionsGroup}
 */
export function createRarityMeterSection() {
  const { rarityMeter } = brk.series.indicators;
  const { all, sth, lth } = brk.series.cohorts.utxo;

  return {
    name: "Rarity Meter",
    tree: [
      .../** @type {const} */ ([
        { key: "full", name: "Full", title: "Bitcoin Rarity Meter: Full" },
        { key: "local", name: "Local", title: "Bitcoin Rarity Meter: Local" },
        { key: "cycle", name: "Cycle", title: "Bitcoin Rarity Meter: Cycle" },
      ]).map((variant) => {
        const meter = rarityMeter[variant.key];
        return {
          name: variant.name,
          title: variant.title,
          top: priceBands(percentileBands(meter), { defaultActive: true }),
          bottom: [
            histogram({
              series: meter.index,
              name: "Index",
              unit: Unit.count,
              colorFn: (value) =>
                /** @type {const} */ ([
                  colors.ratioPct._0_5,
                  colors.ratioPct._1,
                  colors.ratioPct._2,
                  colors.ratioPct._5,
                  colors.transparent,
                  colors.ratioPct._95,
                  colors.ratioPct._98,
                  colors.ratioPct._99,
                  colors.ratioPct._99_5,
                ])[value + 4],
            }),
            baseline({
              series: meter.score,
              name: "Score",
              unit: Unit.count,
              color: [colors.ratioPct._99, colors.ratioPct._1],
              defaultActive: false,
            }),
          ],
        };
      }),
      {
        name: "Components",
        tree: [
          {
            name: "Realized Price",
            title: "Realized Price",
            pattern: all.realized.price,
            legend: "Realized",
            color: colors.realized,
          },
          {
            name: "Capitalized Price",
            title: "Capitalized Price",
            pattern: all.realized.capitalized.price,
            legend: "Capitalized",
            color: colors.capitalized,
          },
          {
            name: "STH RP",
            title: "STH Realized Price",
            pattern: sth.realized.price,
            legend: "Realized",
            color: colors.realized,
          },
          {
            name: "STH CP",
            title: "STH Capitalized Price",
            pattern: sth.realized.capitalized.price,
            legend: "Capitalized",
            color: colors.capitalized,
          },
          {
            name: "LTH RP",
            title: "LTH Realized Price",
            pattern: lth.realized.price,
            legend: "Realized",
            color: colors.realized,
          },
          {
            name: "LTH CP",
            title: "LTH Capitalized Price",
            pattern: lth.realized.capitalized.price,
            legend: "Capitalized",
            color: colors.capitalized,
          },
        ].map((component) => {
          const [, ratioChart] = priceRatioPercentilesTree({
            pattern: component.pattern,
            title: component.title,
            legend: component.legend,
            color: component.color,
            defaultActivePercentiles: true,
          });
          return { ...ratioChart, name: component.name };
        }),
      },
    ],
  };
}
