/**
 * @template {keyof HTMLElementTagNameMap} Tag
 * @param {Tag} tag
 * @param {string} part
 */
export function createWalletPart(tag, part) {
  const element = document.createElement(tag);

  element.dataset.wallet = part;

  return element;
}

/**
 * @param {HTMLButtonElement} button
 * @param {boolean} busy
 * @param {string} idleLabel
 * @param {string} busyLabel
 */
export function setBusy(button, busy, idleLabel, busyLabel) {
  button.disabled = busy;
  button.ariaBusy = busy ? "true" : "false";
  button.textContent = busy ? busyLabel : idleLabel;
}

/**
 * @param {HTMLButtonElement} button
 * @param {string} idleLabel
 * @param {string} busyLabel
 * @param {() => Promise<void>} task
 */
export async function withBusy(button, idleLabel, busyLabel, task) {
  setBusy(button, true, idleLabel, busyLabel);

  try {
    await task();
  } finally {
    setBusy(button, false, idleLabel, busyLabel);
  }
}

/**
 * @param {HTMLElement} status
 * @param {string} text
 */
export function setStatus(status, text) {
  status.textContent = text;
}
