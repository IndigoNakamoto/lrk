import { colors } from "../../utils/colors.js";
import { brk } from "../../utils/client.js";
import { Unit } from "../../utils/units.js";
import { chartsFromCount, line } from "../series.js";
import { groupedWindowsCumulative } from "../shared.js";

const TYPE_DEFINITIONS = /** @type {const} */ ([
  { key: "runes", name: "Runes", defaultActive: true },
  { key: "veriBlock", name: "VeriBlock", defaultActive: true },
  { key: "omni", name: "Omni", defaultActive: true },
  { key: "stacks", name: "Stacks", defaultActive: true },
  { key: "blockstack", name: "Blockstack", defaultActive: false },
  { key: "colu", name: "Colu", defaultActive: false },
  { key: "openAssets", name: "Open Assets", defaultActive: false },
  { key: "komodo", name: "Komodo", defaultActive: false },
  { key: "coinSpark", name: "CoinSpark", defaultActive: false },
  { key: "poet", name: "Po.et", defaultActive: false },
  { key: "docproof", name: "Docproof", defaultActive: false },
  { key: "openTimestamps", name: "OpenTimestamps", defaultActive: true },
  { key: "factom", name: "Factom", defaultActive: false },
  { key: "eternityWall", name: "Eternity Wall", defaultActive: false },
  { key: "memo", name: "Memo", defaultActive: false },
  { key: "bitproof", name: "Bitproof", defaultActive: false },
  { key: "ascribe", name: "Ascribe", defaultActive: false },
  { key: "stampery", name: "Stampery", defaultActive: false },
  { key: "epobc", name: "EPOBC", defaultActive: false },
  { key: "bareHash", name: "Bare Hash", defaultActive: false },
  { key: "text", name: "Text", defaultActive: true },
  { key: "empty", name: "Empty", defaultActive: false },
  { key: "unknown", name: "Unknown", defaultActive: true },
]);

const METRICS = /** @type {const} */ ([
  {
    key: "carrierTxCount",
    name: "Transactions",
    title: "Carrier Transaction Count",
    unit: Unit.count,
  },
  {
    key: "outputCount",
    name: "Outputs",
    title: "Output Count",
    unit: Unit.count,
  },
  {
    key: "carrierVsize",
    name: "vBytes",
    title: "Carrier Transaction vBytes",
    unit: Unit.vb,
  },
  {
    key: "postOpReturnBytes",
    name: "Data",
    title: "Data Bytes",
    unit: Unit.bytes,
  },
]);

/** @typedef {(typeof brk.series.opReturn.byKind)[keyof typeof brk.series.opReturn.byKind]} OpReturnMetrics */

/**
 * @param {Object} args
 * @param {readonly { name: string, color: Color, defaultActive: boolean, pattern: OpReturnMetrics }[]} args.list
 * @param {string} args.category
 * @returns {PartialOptionsTree}
 */
function createComparisons({ list, category }) {
  return METRICS.map((metric) => ({
    name: metric.name,
    tree: groupedWindowsCumulative({
      list,
      title: (title) => `OP_RETURN ${title}`,
      metricTitle: `${metric.title} by ${category}`,
      getWindowSeries: (entry, window) =>
        entry.pattern[metric.key].sum[window],
      getCumulativeSeries: (entry) =>
        entry.pattern[metric.key].cumulative,
      seriesFn: line,
      unit: metric.unit,
    }),
  }));
}

/** @returns {PartialOptionsGroup} */
export function createOpReturnSection() {
  const opReturn = brk.series.opReturn;
  const types = TYPE_DEFINITIONS.map((type, index) => ({
    ...type,
    color: colors.at(index, TYPE_DEFINITIONS.length),
    pattern: opReturn.byKind[type.key],
  }));
  const policies = [
    {
      name: "Standard",
      color: colors.profit,
      defaultActive: true,
      pattern: opReturn.policy.standard,
    },
    {
      name: "Oversized",
      color: colors.loss,
      defaultActive: true,
      pattern: opReturn.policy.oversized,
    },
    {
      name: "Multiple Outputs",
      color: colors.bitcoin,
      defaultActive: true,
      pattern: opReturn.policy.multiple,
    },
    {
      name: "Pre-v30 Nonstandard",
      color: colors.gray,
      defaultActive: true,
      pattern: opReturn.policy.preV30Nonstandard,
    },
  ];

  return {
    name: "OP_RETURN",
    tree: [
      {
        name: "Total",
        tree: [
          {
            name: "Transactions",
            tree: chartsFromCount({
              pattern: opReturn.total.carrierTxCount,
              metric: "OP_RETURN Carrier Transaction Count",
              unit: Unit.count,
              color: colors.scriptType.opReturn,
            }),
          },
          {
            name: "vBytes",
            tree: chartsFromCount({
              pattern: opReturn.total.carrierVsize,
              metric: "OP_RETURN Carrier Transaction vBytes",
              unit: Unit.vb,
              color: colors.scriptType.opReturn,
            }),
          },
          {
            name: "Data",
            tree: chartsFromCount({
              pattern: opReturn.total.postOpReturnBytes,
              metric: "OP_RETURN Data Bytes",
              unit: Unit.bytes,
              color: colors.scriptType.opReturn,
            }),
          },
          {
            name: "Share",
            title: "Cumulative OP_RETURN Data Share of Blockchain Size",
            bottom: [
              line({
                series: opReturn.total.dataShare.percent,
                name: "OP_RETURN Data",
                color: colors.scriptType.opReturn,
                unit: Unit.percentage,
              }),
            ],
          },
        ],
      },
      {
        name: "Type",
        tree: createComparisons({ list: types, category: "Type" }),
      },
      {
        name: "Policy",
        tree: [
          {
            name: "Share",
            title: "Cumulative OP_RETURN Data Share of Blockchain Size by Policy",
            bottom: policies.map(({ name, color, defaultActive, pattern }) =>
              line({
                series: pattern.dataShare.percent,
                name,
                color,
                defaultActive,
                unit: Unit.percentage,
              }),
            ),
          },
          ...createComparisons({ list: policies, category: "Policy" }),
        ],
      },
    ],
  };
}
