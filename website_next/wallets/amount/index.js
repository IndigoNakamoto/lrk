import { renderBtcAmount as renderVisibleBtcAmount } from "../../btc/index.js";
import { redaction } from "../redaction/index.js";

const FIXED_PRIVATE_TEXT = "*****";

/**
 * @typedef {Object} BtcAmountOptions
 * @property {boolean} [signed]
 *
 * @typedef {Object} BtcAmount
 * @property {number} sats
 * @property {boolean} signed
 */

/**
 * @param {HTMLElement} element
 * @param {BtcAmount} amount
 */
function renderBtcAmount(element, amount) {
  element.dataset.btcAmount = "";

  if (redaction.isHidden()) {
    element.textContent = FIXED_PRIVATE_TEXT;
    return;
  }

  renderVisibleBtcAmount(element, amount.sats, amount);
}

/**
 * @template {keyof HTMLElementTagNameMap} Tag
 * @param {Tag} tag
 * @param {number} sats
 * @param {BtcAmountOptions} [options]
 */
export function createBtcAmount(tag, sats, options = {}) {
  const element = document.createElement(tag);
  const amount = {
    sats,
    signed: options.signed === true,
  };

  redaction.addEffect(element, () => renderBtcAmount(element, amount));

  return element;
}
