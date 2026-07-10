import { waitForTransition } from "../utils/transition.js";

/** @param {MouseEvent} event */
function closeOnBackdrop(event) {
  const dialog = /** @type {HTMLDialogElement} */ (event.currentTarget);

  if (event.target === dialog) dialog.close();
}

/**
 * @param {HTMLDialogElement} dialog
 * @param {HTMLElement} host
 */
export function openDialog(dialog, host) {
  dialog.dataset.dialog = "";
  host.append(dialog);
  dialog.addEventListener("close", async () => {
    await waitForTransition();
    dialog.remove();
  }, { once: true });
  dialog.addEventListener("click", closeOnBackdrop);
  dialog.showModal();
}
