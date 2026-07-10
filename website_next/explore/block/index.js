import { createBlockHeader } from "./header/index.js";
import { createMinerPane } from "./miner/index.js";
import { createDifficultyPane } from "./difficulty/index.js";
import { createRewardsPane } from "./rewards/index.js";
import { createTransactionPane } from "./transactions/index.js";
import { createFeeChart } from "./fee-chart/index.js";
import { createBlockPreviewPane } from "./preview/index.js";
import { appendPane } from "./pane.js";
import { createBlockReceipt } from "./receipt/index.js";

function noop() {}

/** @param {string} side */
function createColumn(side) {
  const column = document.createElement("div");

  column.dataset.blockColumn = side;

  return column;
}

export function createBlockDetails() {
  const element = document.createElement("section");
  const receipt = createBlockReceipt();
  const header = createBlockHeader([receipt.button]);
  const content = document.createElement("div");
  let destroyPreview = noop;
  let destroyFeeChart = noop;

  element.id = "block-details";
  element.hidden = true;
  element.append(header.element, content);

  function clearContent() {
    destroyPreview();
    destroyPreview = noop;
    destroyFeeChart();
    destroyFeeChart = noop;

    content.textContent = "";
  }

  /** @param {import("../../modules/brk-client/index.js").BlockInfoV1} block */
  function update(block) {
    const extras = block.extras;

    element.hidden = false;
    header.update(block);
    receipt.update(block);

    clearContent();

    const preview = createBlockPreviewPane(block);
    const feeChart = createFeeChart(extras.feeRange, extras.avgFeeRate);
    const left = createColumn("main");
    const right = createColumn("side");

    destroyPreview = preview.destroy;
    destroyFeeChart = feeChart.destroy;
    appendPane(left, "preview", [
      createTransactionPane(block),
      preview.element,
    ]);
    appendPane(right, "mining", [createMinerPane(block)]);
    appendPane(right, "rewards", [createRewardsPane(extras)]);
    appendPane(right, "difficulty", [createDifficultyPane(block)]);
    appendPane(right, "fees", [feeChart.element]);
    content.append(left, right);
  }

  return /** @type {const} */ ({
    element,
    update,
  });
}
