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
  host.append(dialog);
  dialog.addEventListener("close", () => dialog.remove(), { once: true });
  dialog.addEventListener("click", closeOnBackdrop);
  dialog.showModal();
}
