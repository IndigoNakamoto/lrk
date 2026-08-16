import { txColors } from "../../../../../utils/colors.js";

export const OP_RETURN_KIND_FILTERS = /** @type {const} */ ([
  ["runes", "runes", "runes", txColors.opReturnKind.runes],
  ["veri block", "veri_block", "veriBlock", txColors.opReturnKind.veriBlock],
  ["omni", "omni", "omni", txColors.opReturnKind.omni],
  ["stacks", "stacks", "stacks", txColors.opReturnKind.stacks],
  ["blockstack", "blockstack", "blockstack", txColors.opReturnKind.blockstack],
  ["colu", "colu", "colu", txColors.opReturnKind.colu],
  ["open assets", "open_assets", "openAssets", txColors.opReturnKind.openAssets],
  ["komodo", "komodo", "komodo", txColors.opReturnKind.komodo],
  ["coin spark", "coin_spark", "coinSpark", txColors.opReturnKind.coinSpark],
  ["poet", "poet", "poet", txColors.opReturnKind.poet],
  ["docproof", "docproof", "docproof", txColors.opReturnKind.docproof],
  [
    "open timestamps",
    "open_timestamps",
    "openTimestamps",
    txColors.opReturnKind.openTimestamps,
  ],
  ["factom", "factom", "factom", txColors.opReturnKind.factom],
  [
    "eternity wall",
    "eternity_wall",
    "eternityWall",
    txColors.opReturnKind.eternityWall,
  ],
  ["memo", "memo", "memo", txColors.opReturnKind.memo],
  ["bitproof", "bitproof", "bitproof", txColors.opReturnKind.bitproof],
  ["ascribe", "ascribe", "ascribe", txColors.opReturnKind.ascribe],
  ["stampery", "stampery", "stampery", txColors.opReturnKind.stampery],
  ["epobc", "epobc", "epobc", txColors.opReturnKind.epobc],
  ["bare hash", "bare_hash", "bareHash", txColors.opReturnKind.bareHash],
  ["text", "text", "text", txColors.opReturnKind.text],
  ["empty", "empty", "empty", txColors.opReturnKind.empty],
  ["unknown", "unknown", "unknown", txColors.opReturnKind.unknown],
]);

export const OP_RETURN_POLICY_FILTERS = /** @type {const} */ ([
  ["standard", "standard", "preV30Standard", txColors.opReturnPolicy.standard],
  ["oversized", "oversized", "oversized", txColors.opReturnPolicy.oversized],
  ["multiple", "multiple", "multiple", txColors.opReturnPolicy.multiple],
  [
    "pre-v30 nonstandard",
    "pre_v30_nonstandard",
    "preV30Nonstandard",
    txColors.opReturnPolicy.preV30Nonstandard,
  ],
]);
