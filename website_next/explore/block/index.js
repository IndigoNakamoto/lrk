import { createBlockHeader } from "./header.js";
import { createMinerPane } from "./miner.js";
import { createDifficultyPane } from "./difficulty.js";
import { createRewardsPane } from "./rewards.js";
import { createTransactionPane } from "./transactions.js";
import { createFeeChart } from "./fee-chart.js";
import { appendPane } from "./pane.js";
import { createBlockReceipt } from "./receipt.js";

export function createBlockDetails() {
  const element = document.createElement("section");
  const receipt = createBlockReceipt();
  const header = createBlockHeader([receipt.button]);
  const content = document.createElement("div");

  element.id = "block-details";
  element.hidden = true;
  element.append(header.element, content);

  /** @param {import("../../modules/brk-client/index.js").BlockInfoV1} block */
  function update(block) {
    const extras = block.extras;

    element.hidden = false;
    header.update(block);
    receipt.update(block);

    for (const chart of content.querySelectorAll("[data-fee-chart]")) {
      chart.dispatchEvent(new Event("chart:destroy"));
    }
    content.textContent = "";

    appendPane(content, "mining", [createMinerPane(block)]);
    appendPane(content, "difficulty", [createDifficultyPane(block)]);
    appendPane(content, "rewards", [createRewardsPane(extras)]);
    appendPane(content, "block", [createTransactionPane(block)]);
    appendPane(content, "fees", [
      createFeeChart(extras.feeRange, extras.avgFeeRate),
    ]);
  }

  return /** @type {const} */ ({
    element,
    update,
  });
}
