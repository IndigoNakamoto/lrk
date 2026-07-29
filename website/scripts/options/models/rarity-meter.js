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
  const { all, sth, lth, overAge, underAge } = brk.series.cohorts.utxo;
  const { cointime, coinflow } = brk.series;
  const components = rarityMeter.components;

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
          top: priceBands(percentileBands(meter)),
          bottom: [
            histogram({
              series: meter.index,
              name: "Index",
              unit: Unit.count,
              colorFn: (value) =>
                /** @type {const} */ ([
                  colors.ratioPct._0_01,
                  colors.ratioPct._0_5,
                  colors.ratioPct._1,
                  colors.ratioPct._2,
                  colors.ratioPct._5,
                  colors.transparent,
                  colors.ratioPct._95,
                  colors.ratioPct._98,
                  colors.ratioPct._99,
                  colors.ratioPct._99_5,
                  colors.ratioPct._99_9,
                ])[value + 5],
            }),
            baseline({
              series: meter.score,
              name: "Score",
              unit: Unit.count,
              color: [colors.ratioPct._99_9, colors.ratioPct._0_01],
              defaultActive: false,
            }),
          ],
        };
      }),
      {
        name: "Components",
        tree: [
          {
            name: "RP",
            title: "Realized Price",
            pattern: all.realized.price,
            percentiles: components.realizedPrice,
            legend: "RP",
            color: colors.realized,
          },
          {
            name: "CP",
            title: "Capitalized Price",
            pattern: all.realized.capitalized.price,
            percentiles: components.capitalizedPrice,
            legend: "CP",
            color: colors.capitalized,
          },
          {
            name: "STH RP",
            title: "STH Realized Price",
            pattern: sth.realized.price,
            percentiles: components.sthRealizedPrice,
            legend: "STH RP",
            color: colors.realized,
          },
          {
            name: "STH CP",
            title: "STH Capitalized Price",
            pattern: sth.realized.capitalized.price,
            percentiles: components.sthCapitalizedPrice,
            legend: "STH CP",
            color: colors.capitalized,
          },
          {
            name: "LTH RP",
            title: "LTH Realized Price",
            pattern: lth.realized.price,
            percentiles: components.lthRealizedPrice,
            legend: "LTH RP",
            color: colors.realized,
          },
          {
            name: "LTH CP",
            title: "LTH Capitalized Price",
            pattern: lth.realized.capitalized.price,
            percentiles: components.lthCapitalizedPrice,
            legend: "LTH CP",
            color: colors.capitalized,
          },
          {
            name: ">6M RP",
            title: ">6M Realized Price",
            pattern: overAge._6m.realized.price,
            percentiles: components.over6mRealizedPrice,
            legend: ">6M RP",
            color: colors.realized,
          },
          {
            name: ">4M RP",
            title: ">4M Realized Price",
            pattern: overAge._4m.realized.price,
            percentiles: components.over4mRealizedPrice,
            legend: ">4M RP",
            color: colors.realized,
          },
          {
            name: "<4M RP",
            title: "<4M Realized Price",
            pattern: underAge._4m.realized.price,
            percentiles: components.under4mRealizedPrice,
            legend: "<4M RP",
            color: colors.realized,
          },
          {
            name: "<6M RP",
            title: "<6M Realized Price",
            pattern: underAge._6m.realized.price,
            percentiles: components.under6mRealizedPrice,
            legend: "<6M RP",
            color: colors.realized,
          },
          {
            name: "Vaulted Price",
            title: "Vaulted Price",
            pattern: cointime.prices.vaulted,
            percentiles: components.vaultedPrice,
            legend: "Vaulted",
            color: colors.vaulted,
          },
          {
            name: "Active Price",
            title: "Active Price",
            pattern: cointime.prices.active,
            percentiles: components.activePrice,
            legend: "Active",
            color: colors.active,
          },
          {
            name: "True Market Mean",
            title: "True Market Mean",
            pattern: cointime.prices.trueMarketMean,
            percentiles: components.trueMarketMeanPrice,
            legend: "True Market Mean",
            color: colors.trueMarketMean,
          },
          {
            name: "Cointime Price",
            title: "Cointime Price",
            pattern: cointime.prices.cointime,
            percentiles: components.cointimePrice,
            legend: "Cointime",
            color: colors.cointime,
          },
          {
            name: "Coinflow Price",
            title: "Coinflow Price",
            pattern: coinflow.price,
            percentiles: components.coinflowPrice,
            legend: "Coinflow",
            color: colors.coinflow,
          },
        ].map((component) => {
          const [chart] = priceRatioPercentilesTree({
            pattern: component.pattern,
            percentiles: component.percentiles,
            title: `Bitcoin Rarity Meter: ${component.title}`,
            legend: component.legend,
            color: component.color,
          });
          return { ...chart, name: component.name };
        }),
      },
    ],
  };
}
