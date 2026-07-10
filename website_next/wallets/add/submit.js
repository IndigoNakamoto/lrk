import { withBusy } from "../dom.js";
import { generateAddressesFromWalletSource } from "../derive/index.js";
import { readWalletSourceText } from "./source.js";

/** @typedef {import("./index.js").AddWalletFormSubmit} AddWalletFormSubmit */

/**
 * @param {Object} options
 * @param {{ addWallet(input: { name: string, source: string }): Promise<void> }} options.vault
 * @param {HTMLDialogElement} options.dialog
 * @param {() => void} options.onAdded
 */
export function createAddSubmit({ vault, dialog, onAdded }) {
  /** @param {AddWalletFormSubmit} formData */
  return async function submitWallet({
    name,
    source,
    submit: button,
    form,
  }) {
    await withBusy(button, "Add", "Adding", async () => {
      source.removeAttribute("aria-invalid");

      try {
        const value = readWalletSourceText(source.value);

        await generateAddressesFromWalletSource(value, { count: 1 });

        await vault.addWallet({
          name: name.value,
          source: value,
        });

        form.reset();
        dialog.close();
        onAdded();
      } catch {
        source.setAttribute("aria-invalid", "true");
        source.focus();
      }
    });
  };
}
