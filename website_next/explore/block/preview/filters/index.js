import { createLegendItem } from "../../../../legend/index.js";
import { formatNumber } from "../../format.js";
import { FILTER_GROUP_FILTERS, FILTER_GROUP_LABELS, FILTERS } from "./model.js";

const SUMMARY_LABEL_COUNT = 3;
const CAN_HOVER = matchMedia("(hover: hover) and (pointer: fine)");

/**
 * @param {HTMLButtonElement} button
 * @param {boolean} active
 */
function setActive(button, active) {
  button.setAttribute("aria-pressed", String(active));
  button.toggleAttribute("data-muted", !active);
}

/**
 * @param {HTMLButtonElement} button
 */
function setPending(button) {
  button.disabled = true;
  button.setAttribute("aria-pressed", "false");
  button.removeAttribute("data-muted");
}

/**
 * @param {string[]} labels
 */
function formatSummaryValue(labels) {
  const visible = labels.slice(0, SUMMARY_LABEL_COUNT);
  const hidden = labels.length - visible.length;
  const suffix = hidden > 0 ? ` ... +${hidden}` : "";

  return `${visible.join(", ")}${suffix}`;
}

/**
 * @param {number} disabledMask
 * @param {HTMLElement} value
 */
function updateSummaryValue(disabledMask, value) {
  const labels = FILTERS
    .filter(({ bit }) => disabledMask & bit)
    .map(({ group, label }) => {
      return `${FILTER_GROUP_LABELS.get(group)} ${label}`;
    });

  value.textContent = labels.length > 0
    ? `hidden ${formatSummaryValue(labels)}`
    : "NONE";
}

/**
 * @param {string} label
 */
function createFilterGroup(label) {
  const group = document.createElement("div");
  const title = document.createElement("span");

  title.textContent = label;
  group.append(title);

  return group;
}

/**
 * @param {HTMLElement} panel
 * @param {HTMLElement} summary
 */
function clearGroups(panel, summary) {
  panel.replaceChildren(summary);
}

function createFilterPanel() {
  const panel = document.createElement("details");
  const summary = document.createElement("summary");
  const prefix = document.createElement("span");
  const value = document.createElement("span");

  prefix.textContent = "filters:";
  panel.dataset.blockPreviewFilters = "";
  summary.append(prefix, value);
  panel.append(summary);

  return { panel, summary, summaryValue: value };
}

export function createPendingPreviewFilters() {
  const { panel, summaryValue } = createFilterPanel();

  summaryValue.textContent = "loading";

  return panel;
}

/**
 * @param {() => Promise<BlockPreviewFilterState>} loadFilters
 * @param {BlockPreviewHeatmap} heatmap
 */
export function createPreviewFilters(loadFilters, heatmap) {
  const { panel, summary, summaryValue } = createFilterPanel();
  let disabledMask = 0;
  let live = true;
  let loading = false;
  let state = /** @type {BlockPreviewFilterState | null} */ (null);
  let previewButton = /** @type {HTMLButtonElement | null} */ (null);

  updateSummaryValue(disabledMask, summaryValue);

  function resetPreview() {
    previewButton?.removeAttribute("data-preview");
    previewButton = null;
    heatmap.setPreviewMask(null);
  }

  /**
   * @param {HTMLButtonElement} button
   * @param {number} nextMask
   */
  function preview(button, nextMask) {
    resetPreview();
    previewButton = button;
    button.dataset.preview = "";
    heatmap.setPreviewMask(nextMask);
  }

  /**
   * @param {BlockPreviewFilterState | null} filterState
   */
  function renderFilters(filterState) {
    clearGroups(panel, summary);

    for (const { label, filters } of FILTER_GROUP_FILTERS) {
      const group = createFilterGroup(label);

      for (const filter of filters) {
        const { button, value } = createLegendItem({
          ariaLabel: filter.label,
          color: filter.color,
          label: filter.label,
        });

        value.textContent = filterState === null
          ? "..."
          : formatNumber(filterState.counts[filter.index]);

        if (filterState === null) {
          setPending(button);
        } else {
          const canPreview = CAN_HOVER.matches;

          button.addEventListener("click", () => {
            const active = (disabledMask & filter.bit) === 0;

            resetPreview();
            disabledMask = active
              ? disabledMask | filter.bit
              : disabledMask & ~filter.bit;
            setActive(button, !active);
            updateSummaryValue(disabledMask, summaryValue);
            heatmap.setDisabledMask(disabledMask);
          });

          if (canPreview) {
            button.addEventListener("pointerenter", () => {
              preview(button, filter.bit);
            });
            button.addEventListener("pointerleave", resetPreview);
          }

          button.addEventListener("focus", () => {
            preview(button, filter.bit);
          });
          button.addEventListener("blur", resetPreview);
          setActive(button, (disabledMask & filter.bit) === 0);
        }

        group.append(button);
      }

      panel.append(group);
    }
  }

  function load() {
    if (loading || state !== null) return;

    loading = true;
    summaryValue.textContent = "loading";
    renderFilters(null);
    void loadFilters()
      .then((nextState) => {
        if (!live) return;

        loading = false;
        state = nextState;
        heatmap.setFilterState(nextState);
        updateSummaryValue(disabledMask, summaryValue);
        renderFilters(nextState);
      })
      .catch((error) => {
        if (!live) return;

        loading = false;
        summaryValue.textContent = "unavailable";
        clearGroups(panel, summary);
        console.error(error);
      });
  }

  panel.addEventListener("toggle", () => {
    if (panel.open) load();
  });

  return /** @type {const} */ ({
    destroy() {
      live = false;
      resetPreview();
    },
    element: panel,
  });
}

/** @typedef {import("../data.js").BlockPreviewFilterState} BlockPreviewFilterState */

/**
 * @typedef {Object} BlockPreviewHeatmap
 * @property {(mask: number | null) => void} setPreviewMask
 * @property {(mask: number) => void} setDisabledMask
 * @property {(state: BlockPreviewFilterState) => void} setFilterState
 */
