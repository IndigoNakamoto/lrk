import { createWalletPart } from "../dom.js";

/**
 * @typedef {Object} WalletsLayout
 * @property {HTMLElement} main
 * @property {HTMLElement} utilities
 * @property {HTMLButtonElement} privacyButton
 * @property {HTMLButtonElement} sessionButton
 * @property {HTMLElement} selector
 * @property {HTMLElement} walletList
 * @property {HTMLElement} content
 * @property {HTMLDialogElement} addDialog
 */

/**
 * @returns {WalletsLayout}
 */
export function createLayout() {
  const main = document.createElement("main");
  const utilities = document.createElement("footer");
  const privacyButton = document.createElement("button");
  const sessionButton = document.createElement("button");
  const selector = createWalletPart("section", "selector");
  const walletList = document.createElement("nav");
  const content = document.createElement("article");
  const addDialog = document.createElement("dialog");

  main.dataset.page = "wallets";
  privacyButton.type = "button";
  sessionButton.type = "button";
  sessionButton.append("Lock");
  content.setAttribute("aria-live", "polite");
  walletList.setAttribute("tabindex", "0");
  walletList.setAttribute("aria-label", "Wallets");
  utilities.append(privacyButton, sessionButton);
  selector.append(walletList);
  main.append(selector, content, utilities, addDialog);

  return {
    main,
    utilities,
    privacyButton,
    sessionButton,
    selector,
    walletList,
    content,
    addDialog,
  };
}
