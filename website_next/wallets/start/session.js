import {
  setStatus,
  withBusy,
} from "../dom.js";
import { getErrorMessage } from "../errors.js";
import { createStart } from "./index.js";

/**
 * @param {Object} options
 * @param {Object} options.vault
 * @param {() => void} options.vault.lock
 * @param {() => void} options.vault.reset
 * @param {() => void} options.vault.startEphemeral
 * @param {() => void} options.vault.clearEphemeral
 * @param {(password: string) => Promise<void>} options.vault.setup
 * @param {(password: string) => Promise<void>} options.vault.unlock
 * @param {HTMLElement} options.content
 * @param {() => void} options.onChange
 */
export function createWalletSession({ vault, content, onChange }) {
  function lock() {
    vault.lock();
    onChange();
  }

  function reset() {
    vault.reset();
    onChange();
  }

  function startEphemeral() {
    vault.startEphemeral();
    onChange();
  }

  function clearEphemeral() {
    vault.clearEphemeral();
    onChange();
  }

  /**
   * @param {string} password
   * @param {HTMLButtonElement} button
   * @param {HTMLElement} status
   * @returns {Promise<boolean>}
   */
  async function unlock(password, button, status) {
    let unlocked = false;

    await withBusy(button, "Unlock", "Unlocking", async () => {
      setStatus(status, "");

      try {
        await vault.unlock(password);
        unlocked = true;
        onChange();
      } catch {
        unlocked = false;
      }
    });

    return unlocked;
  }

  /**
   * @param {string} password
   * @param {HTMLButtonElement} button
   * @param {HTMLElement} status
   */
  async function setup(password, button, status) {
    await withBusy(button, "Create", "Creating", async () => {
      setStatus(status, "");

      try {
        await vault.setup(password);
        onChange();
      } catch (error) {
        setStatus(status, getErrorMessage(error));
      }
    });
  }

  /** @param {"create" | "unlock"} mode */
  function renderStart(mode) {
    content.replaceChildren(createStart({
      mode,
      onPassword(password, button, status) {
        return mode === "unlock"
          ? unlock(password, button, status)
          : setup(password, button, status);
      },
      onEphemeral: startEphemeral,
      onReset: mode === "unlock" ? reset : undefined,
    }));
  }

  return /** @type {const} */ ({
    clearEphemeral,
    lock,
    renderStart,
  });
}
