import { createWalletPart } from "../dom.js";
import { bindHold } from "../../utils/hold.js";

/**
 * @param {Object} options
 * @param {string} options.label
 * @param {string} options.title
 * @param {string} [options.variant]
 * @param {() => void} options.onHold
 */
export function createHoldButton(options) {
  const button = createWalletPart("button", "hold");
  const label = document.createElement("span");

  if (options.variant) button.dataset.variant = options.variant;
  button.type = "button";
  button.dataset.label = options.label;
  button.title = options.title;
  label.append(options.label);
  button.append(label);
  bindHold(button, options.onHold);

  return button;
}
