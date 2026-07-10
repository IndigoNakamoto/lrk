import {
  loadBlockPreview,
  loadBlockPreviewFilters,
} from "./data.js";
import {
  createPendingPreviewFilters,
  createPreviewFilters,
} from "./filters/index.js";
import { createBlockPreviewHeatmap } from "./heatmap/index.js";
import { createBlockPreviewInspector } from "./inspector.js";

function noop() {}

/**
 * @param {HTMLElement} body
 * @param {HTMLElement} filters
 * @param {HTMLElement} [inspector]
 */
function createFigure(body, filters, inspector) {
  const figure = document.createElement("figure");
  const caption = document.createElement("figcaption");

  figure.dataset.blockPreviewFigure = "";
  caption.dataset.blockPreviewLegend = "";
  caption.append(filters);
  figure.append(caption, body);
  if (inspector) figure.append(inspector);

  return figure;
}

/**
 * @template T
 * @param {() => Promise<T>} load
 */
function memoize(load) {
  let promise = /** @type {Promise<T> | null} */ (null);

  return () => {
    promise ??= load().catch((error) => {
      promise = null;
      throw error;
    });

    return promise;
  };
}

/**
 * @param {BlockPreviewData} data
 * @param {() => Promise<BlockPreviewFilterState>} loadFilters
 * @param {AbortSignal} signal
 */
function createPreview(data, loadFilters, signal) {
  const loadFilterState = memoize(loadFilters);
  const inspector = createBlockPreviewInspector(signal, loadFilterState);
  const heatmap = createBlockPreviewHeatmap(data, {
    onInspect: inspector.inspect,
  });
  const filters = createPreviewFilters(loadFilterState, heatmap);

  return {
    destroy() {
      inspector.destroy();
      filters.destroy();
      heatmap.destroy();
    },
    element: createFigure(heatmap.element, filters.element, inspector.element),
  };
}

/**
 * @param {HTMLElement} content
 * @param {string} status
 */
function renderStatus(content, status) {
  const p = document.createElement("p");

  p.dataset.blockPreviewStatus = status;
  p.textContent = status;
  content.replaceChildren(createFigure(p, createPendingPreviewFilters()));
}

/**
 * @param {import("../../../modules/brk-client/index.js").BlockInfoV1} block
 */
export function createBlockPreviewPane(block) {
  const content = document.createElement("div");
  const controller = new AbortController();
  let destroyHeatmap = noop;
  let live = true;

  content.dataset.blockPreview = "";
  renderStatus(content, "Loading");

  void loadBlockPreview(block, controller.signal)
    .then((data) => {
      if (!live) return;
      const preview = createPreview(
        data,
        () => loadBlockPreviewFilters(data.range, controller.signal),
        controller.signal,
      );

      destroyHeatmap = preview.destroy;
      content.replaceChildren(preview.element);
    })
    .catch((error) => {
      if (!live) return;
      console.error(error);
      renderStatus(content, "Unavailable");
    });

  return {
    element: content,
    destroy() {
      live = false;
      controller.abort();
      destroyHeatmap();
      destroyHeatmap = noop;
    },
  };
}

/** @typedef {import("./data.js").BlockPreviewData} BlockPreviewData */
/** @typedef {import("./data.js").BlockPreviewFilterState} BlockPreviewFilterState */

/**
 * @typedef {Object} BlockPreview
 * @property {() => void} destroy
 * @property {HTMLElement} element
 */
