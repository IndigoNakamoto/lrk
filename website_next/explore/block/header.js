import { createUsdAmount, renderUsdAmount } from "../../usd/index.js";
import { formatDateTime } from "./format.js";
import { createBlockTitle } from "./title.js";

/** @typedef {import("../../modules/brk-client/index.js").BlockInfoV1} Block */

/** @param {string} hash */
function createHashElement(hash) {
  const element = document.createElement("span");
  const prefix = document.createElement("span");
  const value = document.createElement("span");
  const firstNonZero = hash.search(/[^0]/);
  const visibleStart = firstNonZero === -1 ? hash.length : firstNonZero;

  element.dataset.blockHash = "";
  prefix.dataset.dim = "";
  prefix.textContent = hash.slice(0, visibleStart);
  value.textContent = hash.slice(visibleStart);
  element.append(prefix, value);

  return element;
}

/** @param {Node[]} [actions] */
export function createBlockHeader(actions = []) {
  const element = document.createElement("header");
  const titleRow = document.createElement("div");
  const main = document.createElement("div");
  const title = document.createElement("h1");
  const side = document.createElement("div");
  const actionList = document.createElement("div");
  const date = document.createElement("time");
  const hash = document.createElement("p");
  const price = createUsdAmount("output", 0, {
    tone: "positive",
  });

  main.dataset.blockMain = "";
  titleRow.dataset.blockTitle = "";
  side.dataset.blockSide = "";
  actionList.dataset.blockActions = "";
  date.dataset.blockDate = "";
  hash.dataset.blockHashLine = "";
  actionList.append(...actions);
  side.append(date, price, actionList);
  main.append(title, hash);
  titleRow.append(main, side);
  element.append(titleRow);

  /** @param {Block} block */
  function update(block) {
    title.replaceChildren(...createBlockTitle(block.height));
    date.dateTime = new Date(block.timestamp * 1_000).toISOString();
    date.textContent = formatDateTime(block.timestamp);
    hash.replaceChildren(createHashElement(block.id));
    renderUsdAmount(price, block.extras.price, {
      tone: "positive",
    });
  }

  return /** @type {const} */ ({
    element,
    update,
  });
}
