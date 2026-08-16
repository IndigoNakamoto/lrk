import { txColors } from "../../../../utils/colors.js";
import {
  OP_RETURN_KIND_FILTERS,
  OP_RETURN_POLICY_FILTERS,
} from "./op-return/model.js";

export const FILTER_GROUPS = /** @type {const} */ ([
  { key: "version", label: "version" },
  { key: "rbf", label: "rbf" },
  { key: "input", label: "input" },
  { key: "output", label: "output" },
  { key: "type", label: "type" },
  { key: "behavior", label: "behavior" },
  { key: "data", label: "data" },
  { key: "sighash", label: "sighash" },
  { key: "policy", label: "policy" },
  { key: "op_return", label: "op return" },
]);

const FILTER_DEFS = /** @type {const} */ ([
  ["version", "v1", "version:1", txColors.v1],
  ["version", "v2", "version:2", txColors.v2],
  ["version", "v3", "version:3", txColors.v3],
  ["version", "other", "version:other", txColors.otherVersion],
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
  [
    "behavior",
    "paid by child",
    "behavior:cpfp_parent",
    txColors.behavior.cpfpParent,
  ],
  [
    "behavior",
    "pays parent",
    "behavior:cpfp_child",
    txColors.behavior.cpfpChild,
  ],
  ["behavior", "coinjoin", "behavior:coinjoin", txColors.behavior.coinjoin],
  [
    "behavior",
    "consolidation",
    "behavior:consolidation",
    txColors.behavior.consolidation,
  ],
  ["behavior", "batch", "behavior:batch", txColors.behavior.batchPayout],
  ["data", "fake pubkey", "data:fake_pubkey", txColors.data.fakePubkey],
  [
    "data",
    "fake scripthash",
    "data:fake_scripthash",
    txColors.data.fakeScripthash,
  ],
  ["data", "inscription", "data:inscription", txColors.data.inscription],
  ["data", "annex", "data:annex", txColors.data.annex],
  ["data", "dust", "data:dust", txColors.data.dust],
  ["sighash", "all", "sighash:all", txColors.sighash.all],
  ["sighash", "none", "sighash:none", txColors.sighash.none],
  ["sighash", "single", "sighash:single", txColors.sighash.single],
  ["sighash", "default", "sighash:default", txColors.sighash.default],
  [
    "sighash",
    "anyonecanpay",
    "sighash:anyone_can_pay",
    txColors.sighash.anyoneCanPay,
  ],
  [
    "policy",
    "nonstandard",
    "policy:nonstandard",
    txColors.policy.nonstandard,
  ],
  ...OP_RETURN_KIND_FILTERS.map(([label, kind, , color]) => {
    return /** @type {const} */ (["op_return", label, `op_return:${kind}`, color]);
  }),
  ...OP_RETURN_POLICY_FILTERS.map(([label, policy, , color]) => {
    return /** @type {const} */ ([
      "op_return",
      label,
      `op_return_policy:${policy}`,
      color,
    ]);
  }),
]);

export const FILTERS = FILTER_DEFS.map(([group, label, key, color], index) => {
  return /** @type {const} */ ({ color, group, index, key, label });
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

/** @typedef {(typeof FILTERS)[number]} BlockPreviewFilter */
