import { createQrDataUrl } from "../../../qr/index.js";
import { openDialog } from "../../../dialog/index.js";
import { createGroupedAddress } from "../address/index.js";
import { createWalletPart } from "../../dom.js";
import { formatNumber } from "../../format.js";

/**
 * @typedef {import("../../scan/index.js").WalletAddress} ReceiveAddress
 */

/**
 * @param {ReceiveAddress} receiveAddress
 */
function createReceiveTitle(receiveAddress) {
  const title = document.createElement("h2");

  title.append(
    `${receiveAddress.branchLabel.toLowerCase()} #${formatNumber(receiveAddress.index)}`,
  );

  return title;
}

/**
 * @param {ReceiveAddress} receiveAddress
 */
function createReceiveQr(receiveAddress) {
  const image = document.createElement("img");
  const uri = `bitcoin:${receiveAddress.address}`;

  image.alt = `QR code for ${receiveAddress.address}`;
  image.src = createQrDataUrl(uri, { scale: 8 });

  return image;
}

/**
 * @param {ReceiveAddress} receiveAddress
 */
function createReceiveAddress(receiveAddress) {
  const element = document.createElement("p");

  element.append(createGroupedAddress(receiveAddress.address));

  return element;
}

/**
 * @param {ReceiveAddress} receiveAddress
 * @param {HTMLButtonElement} copy
 */
async function copyReceiveAddress(receiveAddress, copy) {
  await navigator.clipboard.writeText(receiveAddress.address);
  copy.textContent = "Copied";
}

/**
 * @param {HTMLElement} host
 * @param {ReceiveAddress} receiveAddress
 */
function openReceiveDialog(host, receiveAddress) {
  const dialog = createWalletPart("dialog", "receive");
  const content = document.createElement("article");
  const actions = document.createElement("footer");
  const copy = document.createElement("button");
  const closeForm = document.createElement("form");
  const close = document.createElement("button");

  copy.type = "button";
  copy.append("Copy");
  closeForm.method = "dialog";
  close.type = "submit";
  close.append("Close");
  closeForm.append(close);
  actions.append(copy, closeForm);
  content.append(
    createReceiveTitle(receiveAddress),
    createReceiveQr(receiveAddress),
    createReceiveAddress(receiveAddress),
    actions,
  );
  dialog.append(content);

  copy.addEventListener("click", () => {
    void copyReceiveAddress(receiveAddress, copy).catch(() => {
      copy.textContent = "Copy failed";
    });
  });
  openDialog(dialog, host);
}

/**
 * @param {HTMLElement} element
 * @param {ReceiveAddress | undefined} receiveAddress
 */
export function renderReceiveButton(element, receiveAddress) {
  const button = document.createElement("button");

  button.type = "button";
  button.disabled = !receiveAddress;
  button.append("Receive");
  button.addEventListener("click", () => {
    if (receiveAddress) {
      openReceiveDialog(element, receiveAddress);
    }
  });
  element.append(button);
}
