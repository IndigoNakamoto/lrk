import { formatBlockFill, formatBytes, formatNumber } from "./format.js";

/** @typedef {import("../../modules/brk-client/index.js").BlockInfoV1} Block */

/**
 * @param {string} label
 * @param {(string | Node)[]} values
 */
function createBlockRow(label, values) {
  const row = document.createElement("div");
  const name = document.createElement("span");
  const data = document.createElement("strong");

  row.dataset.blockRow = "";
  name.textContent = label;
  data.append(...values);
  row.append(name, data);

  return row;
}

/**
 * @param {string} label
 * @param {string | Node} value
 * @param {string} type
 */
function createBlockBox(label, value, type) {
  const box = document.createElement("div");

  box.dataset.blockBox = type;
  box.append(createBlockRow(label, [value]));

  return box;
}

/** @param {Block} block */
export function createTransactionPane(block) {
  const { extras } = block;
  const box = document.createElement("div");
  const transactions = document.createElement("div");
  const io = document.createElement("div");

  box.dataset.blockBox = "";
  transactions.dataset.blockBox = "tx";
  io.dataset.blockIo = "";
  io.append(
    createBlockBox("Input", formatNumber(extras.totalInputs), "input"),
    createBlockBox("Output", formatNumber(extras.totalOutputs), "output"),
  );
  transactions.append(
    createBlockRow("Tx", [formatNumber(block.txCount)]),
    io,
  );
  box.append(
    createBlockRow("Block", [
      `${formatBytes(block.size)} · ${formatBlockFill(block.weight)}`,
    ]),
    transactions,
  );

  return box;
}
