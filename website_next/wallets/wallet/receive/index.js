import { createQrDataUrl } from "../../../qr/index.js";
import { openDialog } from "../../../dialog/index.js";
import { createGroupedAddress } from "../address/index.js";
import { createWalletPart } from "../../dom.js";

/**
 * @typedef {import("../../scan/index.js").WalletAddress} ReceiveAddress
 */

/**
 * @param {ReceiveAddress} receiveAddress
 */
function createReceiveQr(receiveAddress) {
  const image = document.createElement("img");
  const uri = `bitcoin:${receiveAddress.address}`;

  image.alt = `QR code for ${receiveAddress.address}`;
  image.src = createQrDataUrl(uri, { scale: 6 });

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
 * @param {HTMLElement} content
 * @param {() => void} onCopied
 */
async function copyReceiveAddress(receiveAddress, content, onCopied) {
  await navigator.clipboard.writeText(receiveAddress.address);
  content.dataset.copied = "";
  onCopied();
}

/**
 * @param {HTMLElement} host
 * @param {ReceiveAddress} receiveAddress
 */
function openReceiveDialog(host, receiveAddress) {
  const dialog = createWalletPart("dialog", "receive");
  const content = document.createElement("article");
  let copiedTimeout = 0;

  content.role = "button";
  content.tabIndex = 0;
  content.append(
    createReceiveQr(receiveAddress),
    createReceiveAddress(receiveAddress),
  );
  dialog.append(content);

  function copy() {
    void copyReceiveAddress(receiveAddress, content, () => {
      window.clearTimeout(copiedTimeout);
      copiedTimeout = window.setTimeout(() => {
        delete content.dataset.copied;
      }, 1_000);
    }).catch(() => {});
  }

  content.addEventListener("click", copy);
  content.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;

    event.preventDefault();
    copy();
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
