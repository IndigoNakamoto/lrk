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
 * @param {string[]} labels
 */
function formatSummaryValue(labels) {
  const visible = labels.slice(0, SUMMARY_LABEL_COUNT);
  const hidden = labels.length - visible.length;
  const suffix = hidden > 0 ? ` ... +${hidden}` : "";

  return `${visible.join(", ")}${suffix}`;
}

/**
 * @param {Set<string>} hiddenKeys
 * @param {HTMLElement} value
 */
function updateSummaryValue(hiddenKeys, value) {
  const labels = FILTERS
    .filter(({ key }) => hiddenKeys.has(key))
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
 * @param {BlockPreviewFilterData} data
 * @param {BlockPreviewHeatmap} heatmap
 */
export function createPreviewFilters(data, heatmap) {
  const { panel, summary, summaryValue } = createFilterPanel();
  const appliedKeys = new Set();
  const hiddenKeys = new Set();
  let live = true;
  let loading = false;
  let counts = /** @type {Uint32Array | null} */ (null);
  let previewButton = /** @type {HTMLButtonElement | null} */ (null);

  updateSummaryValue(hiddenKeys, summaryValue);

  function resetPreview() {
    previewButton?.removeAttribute("data-preview");
    previewButton = null;
    heatmap.setPreviewMembership(null);
  }

  /**
   * @param {HTMLButtonElement} button
   * @param {BlockPreviewFilter} filter
   */
  function preview(button, filter) {
    resetPreview();
    previewButton = button;
    button.setAttribute("aria-busy", "true");

    void data.loadMembership(filter)
      .then((membership) => {
        button.removeAttribute("aria-busy");
        if (!live || previewButton !== button) return;

        button.dataset.preview = "";
        heatmap.setPreviewMembership(membership);
      })
      .catch((error) => {
        button.removeAttribute("aria-busy");
        if (!live) return;

        resetPreview();
        console.error(error);
      });
  }

  /**
   * @param {BlockPreviewFilter} filter
   * @param {Uint8Array} membership
   */
  function syncHidden(filter, membership) {
    const hidden = hiddenKeys.has(filter.key);
    const applied = appliedKeys.has(filter.key);

    if (hidden === applied) return;

    heatmap.setFilterHidden(membership, hidden);
    if (hidden) appliedKeys.add(filter.key);
    else appliedKeys.delete(filter.key);
  }

  /** @param {Uint32Array} filterCounts */
  function renderFilters(filterCounts) {
    clearGroups(panel, summary);
    const canPreview = CAN_HOVER.matches;

    for (const { label, filters } of FILTER_GROUP_FILTERS) {
      let group = /** @type {HTMLDivElement | null} */ (null);

      for (const filter of filters) {
        const count = filterCounts[filter.index];
        if (count === 0) continue;

        group ??= createFilterGroup(label);
        const { button, value } = createLegendItem({
          ariaLabel: filter.label,
          color: filter.color,
          label: filter.label,
        });

        value.textContent = formatNumber(count);
        button.addEventListener("click", () => {
          resetPreview();
          if (hiddenKeys.has(filter.key)) hiddenKeys.delete(filter.key);
          else hiddenKeys.add(filter.key);

          setActive(button, !hiddenKeys.has(filter.key));
          updateSummaryValue(hiddenKeys, summaryValue);
          void data.loadMembership(filter)
            .then((membership) => {
              if (live) syncHidden(filter, membership);
            })
            .catch(console.error);
        });

        if (canPreview) {
          button.addEventListener("pointerenter", () => {
            preview(button, filter);
          });
          button.addEventListener("pointerleave", resetPreview);
        }

        button.addEventListener("focus", () => {
          preview(button, filter);
        });
        button.addEventListener("blur", resetPreview);
        setActive(button, !hiddenKeys.has(filter.key));
        group.append(button);
      }

      if (group !== null) panel.append(group);
    }
  }

  function load() {
    if (loading || counts !== null) return;

    loading = true;
    summaryValue.textContent = "loading";
    void data.loadCounts()
      .then((nextCounts) => {
        if (!live) return;

        loading = false;
        counts = nextCounts;
        updateSummaryValue(hiddenKeys, summaryValue);
        renderFilters(nextCounts);
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
    else resetPreview();
  });

  return /** @type {const} */ ({
    destroy() {
      live = false;
      resetPreview();
    },
    element: panel,
  });
}

/** @typedef {import("./data.js").BlockPreviewFilterData} BlockPreviewFilterData */
/** @typedef {import("./model.js").BlockPreviewFilter} BlockPreviewFilter */

/**
 * @typedef {Object} BlockPreviewHeatmap
 * @property {(membership: Uint8Array | null) => void} setPreviewMembership
 * @property {(membership: Uint8Array, hidden: boolean) => void} setFilterHidden
 */
