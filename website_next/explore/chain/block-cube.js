import { createPoolLogo, getPoolDisplayName } from "../../pools/index.js";
import { formatFeeRate } from "../../utils/fee-rate.js";
import { createCubeButton, createCubeDiv } from "./cube/index.js";
import { onPlainClick } from "./events.js";
import {
  createHeightElement,
  dim,
  formatHHMM,
  formatNumber,
  formatShortDate,
} from "./format.js";

/** @typedef {import("../../modules/brk-client/index.js").BlockInfoV1} Block */
/** @typedef {import("../../modules/brk-client/index.js").MempoolBlock} MempoolBlock */

export function createPlaceholderCube() {
  const cube = document.createElement("div");

  cube.dataset.cube = "block";
  cube.dataset.placeholder = "";

  return cube;
}

/**
 * @param {Block} block
 * @param {(cube: HTMLButtonElement) => void} onSelect
 */
export function createEnteringConfirmedCube(block, onSelect) {
  const cube = createConfirmedCube(block, onSelect);

  markCubeEntering(cube);

  return cube;
}

/**
 * @param {Block} block
 * @param {(cube: HTMLButtonElement) => void} onSelect
 */
function createConfirmedCube(block, onSelect) {
  const { pool, medianFee, feeRange, virtualSize } = block.extras;
  const cube = createCubeButton(Math.min(1, virtualSize / 1_000_000));

  cube.element.dataset.hash = block.id;
  cube.element.dataset.height = String(block.height);
  cube.element.dataset.timestamp = String(block.timestamp);
  cube.element.title = `Block ${formatNumber(block.height)}`;
  onPlainClick(cube.element, () => onSelect(cube.element));

  const date = document.createElement("p");
  const time = document.createElement("p");
  const [hh, mm] = formatHHMM(block.timestamp);
  date.textContent = formatShortDate(block.timestamp);
  time.append(hh, dim(":"), mm);
  cube.topFace.append(date, time);

  const height = document.createElement("p");
  height.dataset.cubeHeight = "";
  height.append(createHeightElement(block.height));

  const poolElement = document.createElement("div");
  const logo = createPoolLogo(pool);
  const name = document.createElement("span");
  poolElement.dataset.cubePool = "";
  name.textContent = getPoolDisplayName(pool.name);
  poolElement.append(logo, name);
  cube.rightFace.append(height, poolElement);

  const fees = document.createElement("div");
  const median = document.createElement("p");
  const range = document.createElement("p");
  const unit = document.createElement("p");
  fees.dataset.cubeFees = "";
  median.append(dim("~"), formatFeeRate(medianFee));
  range.append(
    formatFeeRate(feeRange[0]),
    dim("-"),
    formatFeeRate(feeRange[6]),
  );
  unit.dataset.dim = "";
  unit.textContent = "sat/vB";
  fees.append(median, range, unit);
  cube.leftFace.append(fees);

  return cube.element;
}

/** @param {HTMLElement} cube */
function markCubeEntering(cube) {
  cube.dataset.enter = "";
  cube.addEventListener(
    "animationend",
    () => {
      cube.removeAttribute("data-enter");
    },
    { once: true },
  );
}

export function createProjectedCube() {
  const cube = createCubeDiv();
  const date = document.createTextNode("");
  const hh = document.createTextNode("");
  const mm = document.createTextNode("");
  const txs = document.createTextNode("");
  const txsUnit = document.createTextNode("");
  const median = document.createTextNode("");
  const rangeLo = document.createTextNode("");
  const rangeHi = document.createTextNode("");

  const dateElement = document.createElement("p");
  const timeElement = document.createElement("p");
  const txsElement = document.createElement("p");
  const txsUnitElement = document.createElement("p");
  const medianElement = document.createElement("p");
  const rangeElement = document.createElement("p");
  const unitElement = document.createElement("p");

  cube.element.dataset.projected = "";
  dateElement.append(date);
  timeElement.append(hh, dim(":"), mm);
  cube.topFace.append(dateElement, timeElement);

  txsElement.append(txs);
  txsUnitElement.dataset.dim = "";
  txsUnitElement.append(txsUnit);
  cube.rightFace.append(txsElement, txsUnitElement);

  medianElement.append(dim("~"), median);
  rangeElement.append(rangeLo, dim("-"), rangeHi);
  unitElement.dataset.dim = "";
  unitElement.textContent = "sat/vB";
  cube.leftFace.append(medianElement, rangeElement, unitElement);

  return {
    ...cube,
    parts: { date, hh, mm, txs, txsUnit, median, rangeLo, rangeHi },
  };
}

/** @param {ReturnType<typeof createProjectedCube>} cube @param {MempoolBlock} block */
export function updateProjectedCube(cube, block) {
  cube.element.style.setProperty(
    "--fill",
    String(Math.min(1, block.blockVSize / 1_000_000)),
  );

  cube.parts.txs.nodeValue = formatNumber(block.nTx);
  cube.parts.txsUnit.nodeValue = block.nTx === 1 ? "tx" : "txs";
  cube.parts.median.nodeValue = formatFeeRate(block.medianFee);
  cube.parts.rangeLo.nodeValue = formatFeeRate(block.feeRange[0]);
  cube.parts.rangeHi.nodeValue = formatFeeRate(block.feeRange[6]);
}

/** @param {ReturnType<typeof createProjectedCube>} cube @param {number} timestamp */
export function updateProjectedTime(cube, timestamp) {
  const [hh, mm] = formatHHMM(timestamp);

  cube.parts.date.nodeValue = formatShortDate(timestamp);
  cube.parts.hh.nodeValue = hh;
  cube.parts.mm.nodeValue = mm;
}

/** @param {HTMLElement} cube */
export function setConfirmedInterval(cube) {
  const prev = /** @type {HTMLElement | null} */ (cube.previousElementSibling);
  if (!prev?.dataset.timestamp) return;

  cube.style.setProperty(
    "--block-interval",
    String(
      Math.max(
        0,
        Number(cube.dataset.timestamp) - Number(prev.dataset.timestamp),
      ),
    ),
  );
}
