import { ASK_MODEL } from "../models.js";

/** @param {number} bytes */
function formatBytes(bytes) {
  return bytes >= 1_000_000
    ? `${(bytes / 1_000_000).toFixed(0)} MB`
    : `${(bytes / 1_000).toFixed(0)} KB`;
}

/** @param {{ onLoad: () => void }} options */
export function createAskLoader(options) {
  const loader = document.createElement("section");
  const load = document.createElement("button");
  const label = document.createElement("span");
  const detail = document.createElement("small");
  const progress = document.createElement("progress");
  const status = document.createElement("p");

  loader.dataset.askLoader = "";
  load.type = "button";
  load.addEventListener("click", options.onLoad);
  label.append("Download & use");
  detail.append(ASK_MODEL.size);
  load.append(label, detail);
  progress.max = 100;
  progress.hidden = true;
  status.role = "status";
  status.ariaLive = "polite";
  status.hidden = true;
  loader.append(load, progress, status);

  /** @param {string} message */
  function showStatus(message) {
    status.hidden = !message;
    status.textContent = message;
  }

  return {
    element: loader,
    checking() {
      loader.hidden = false;
      load.hidden = true;
      progress.hidden = false;
      progress.removeAttribute("value");
      showStatus("Checking browser cache...");
    },
    /** @param {string | undefined} [message] */
    unloaded(message) {
      loader.hidden = false;
      load.hidden = false;
      load.disabled = false;
      label.textContent = "Download & use";
      detail.textContent = ASK_MODEL.size;
      progress.hidden = true;
      showStatus(message ?? "");
    },
    /** @param {boolean} cached */
    loading(cached) {
      loader.hidden = false;
      load.hidden = true;
      progress.hidden = false;
      if (cached) progress.removeAttribute("value");
      else progress.value = 0;
      showStatus(
        cached
          ? "Loading the model from browser storage..."
          : "Preparing the one-time local download...",
      );
    },
    /** @param {string} message */
    loadError(message) {
      loader.hidden = false;
      load.hidden = false;
      load.disabled = false;
      label.textContent = "Try again";
      progress.hidden = true;
      showStatus(`Load failed · ${message}`);
    },
    ready() {
      loader.hidden = true;
    },
    /** @param {string} message */
    answerError(message) {
      loader.hidden = false;
      load.hidden = true;
      progress.hidden = true;
      showStatus(`Could not answer · ${message}`);
    },
    /**
     * @param {{ progress: number, loaded: number, total: number }} update
     * @param {boolean} cached
     */
    reportProgress(update, cached) {
      if (!cached) {
        progress.hidden = false;
        progress.value = update.progress;
      }
      showStatus(
        update.total
          ? cached
            ? "Loading the model from browser storage..."
            : `Downloading ${formatBytes(update.loaded)} of ${formatBytes(update.total)}...`
          : cached
            ? "Starting the cached model..."
            : `Downloading model... ${update.progress.toFixed(0)}%`,
      );
    },
    /**
     * @param {string} message
     * @param {boolean} cached
     */
    reportStatus(message, cached) {
      showStatus(
        cached && message.includes("Downloading")
          ? "Loading the model from browser storage..."
          : message,
      );
    },
  };
}
