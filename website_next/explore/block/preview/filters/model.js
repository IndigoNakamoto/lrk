import { txColors } from "../../../../utils/colors.js";

export const FILTER_GROUPS = /** @type {const} */ ([
  { key: "version", label: "version" },
  { key: "rbf", label: "rbf" },
  { key: "input", label: "input" },
  { key: "output", label: "output" },
  { key: "type", label: "type" },
]);

const FILTER_DEFS = /** @type {const} */ ([
  ["version", "v1", "version:1", txColors.v1],
  ["version", "v2", "version:2", txColors.v2],
  ["version", "v3", "version:3", txColors.v3],
  ["rbf", "yes", "rbf:yes", txColors.rbf],
  ["rbf", "no", "rbf:no", txColors.noRbf],
  ["input", "1", "input:one", txColors.oneInput],
  ["input", "multi", "input:multi", txColors.multiInput],
  ["output", "1", "output:one", txColors.oneOutput],
  ["output", "multi", "output:multi", txColors.multiOutput],
  ["type", "p2pk", "type:p2pk", txColors.p2pk],
  ["type", "p2pkh", "type:p2pkh", txColors.p2pkh],
  ["type", "p2sh", "type:p2sh", txColors.p2sh],
  ["type", "p2wpkh", "type:p2wpkh", txColors.p2wpkh],
  ["type", "p2wsh", "type:p2wsh", txColors.p2wsh],
  ["type", "taproot", "type:taproot", txColors.taproot],
  ["type", "p2a", "type:p2a", txColors.p2a],
  ["type", "baremult", "type:multisig", txColors.baremult],
  ["type", "op ret", "type:op_return", txColors.opReturn],
  ["type", "empty", "type:empty", txColors.empty],
  ["type", "unknown", "type:unknown", txColors.unknown],
]);

export const FILTERS = FILTER_DEFS.map(([group, label, key, color], index) => {
  return /** @type {const} */ ({ bit: 1 << index, color, group, index, key, label });
});

export const FILTER_GROUP_FILTERS = FILTER_GROUPS.map((group) => {
  return /** @type {const} */ ({
    ...group,
    filters: FILTERS.filter((filter) => filter.group === group.key),
  });
});

export const FILTER_GROUP_LABELS = new Map(FILTER_GROUPS.map(({ key, label }) => {
  return [key, label];
}));

const FILTER_BITS = /** @type {Map<string, number>} */ (
  new Map(FILTERS.map(({ bit, key }) => [key, bit]))
);

export const TYPE_FILTER_MASK = FILTERS
  .filter(({ group }) => group === "type")
  .reduce((mask, { bit }) => mask | bit, 0);

export const TYPE_BITS = /** @type {const} */ ({
  empty: getFilterBit("type:empty"),
  multisig: getFilterBit("type:multisig"),
  op_return: getFilterBit("type:op_return"),
  p2a: getFilterBit("type:p2a"),
  p2pk: getFilterBit("type:p2pk"),
  p2pkh: getFilterBit("type:p2pkh"),
  p2sh: getFilterBit("type:p2sh"),
  unknown: getFilterBit("type:unknown"),
  v0_p2wpkh: getFilterBit("type:p2wpkh"),
  v0_p2wsh: getFilterBit("type:p2wsh"),
  v1_p2tr: getFilterBit("type:taproot"),
});

/**
 * @param {string} key
 */
export function getFilterBit(key) {
  return FILTER_BITS.get(key) ?? 0;
}
