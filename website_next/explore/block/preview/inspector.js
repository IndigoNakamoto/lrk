import { formatFeeRate } from "../../../utils/fee-rate.js";
import { formatNumber, formatWeight } from "../format.js";
import { FILTER_GROUP_FILTERS } from "./filters/model.js";
import { placeReadout } from "./inspector/position.js";
import { createTxidLoader } from "./inspector/txid.js";

const TRAITS_DWELL_MS = 180;

/**
 * @param {HTMLElement} parent
 * @param {string} label
 */
function appendField(parent, label) {
  const row = document.createElement("div");
  const term = document.createElement("dt");
  const value = document.createElement("dd");

  term.textContent = label;
  row.append(term, value);
  parent.append(row);

  return value;
}

/**
 * @param {HTMLElement} element
 * @param {string} value
 * @param {boolean} loading
 */
function setField(element, value, loading = false) {
  element.textContent = value;

  if (loading) element.setAttribute("aria-busy", "true");
  else element.removeAttribute("aria-busy");
}

/**
 * @param {AbortSignal} parentSignal
 * @param {BlockPreviewFilterData} filterData
 */
export function createBlockPreviewInspector(parentSignal, filterData) {
  const element = document.createElement("dl");
  const txid = appendField(element, "txid");
  const tx = appendField(element, "tx");
  const fee = appendField(element, "fee");
  const weight = appendField(element, "weight");
  const traitFields = FILTER_GROUP_FILTERS.map(({ filters, label }) => {
    return /** @type {const} */ ({ filters, value: appendField(element, label) });
  });
  const txidLoader = createTxidLoader(parentSignal);
  let inspected = /** @type {BlockPreviewTransaction | null} */ (null);
  let point = /** @type {BlockPreviewPointer | null} */ (null);
  let readoutSize = /** @type {BlockPreviewReadoutSize | null} */ (null);
  let traitsController = /** @type {AbortController | null} */ (null);
  let traitsTimer = 0;

  element.dataset.blockPreviewTransaction = "";
  element.hidden = true;

  function abortPending() {
    clearTimeout(traitsTimer);
    traitsController?.abort();
    traitsController = null;
    txidLoader.abort();
  }

  function invalidateReadoutSize() {
    readoutSize = null;
  }

  /**
   * @param {BlockPreviewPointer} nextPoint
   */
  function place(nextPoint) {
    readoutSize ??= {
      height: element.offsetHeight,
      width: element.offsetWidth,
    };
    placeReadout(element, nextPoint, readoutSize);
  }

  function setTraitsLoading() {
    invalidateReadoutSize();

    for (const { value } of traitFields) {
      value.removeAttribute("title");
      setField(value, "loading", true);
    }
  }

  /** @param {string[]} keys */
  function setTraits(keys) {
    invalidateReadoutSize();
    const activeKeys = new Set(keys);

    for (const { filters, value } of traitFields) {
      const labels = filters
        .filter(({ key }) => activeKeys.has(key))
        .map(({ label }) => label);
      const text = labels.join(" · ") || "none";

      setField(value, text);
      value.title = text;
    }
  }

  /**
   * @param {BlockPreviewTransaction} transaction
   * @param {boolean} eager
   */
  function loadTraits(transaction, eager) {
    setTraitsLoading();
    traitsTimer = setTimeout(() => {
      const controller = new AbortController();
      const signal = AbortSignal.any([parentSignal, controller.signal]);

      traitsController = controller;
      void filterData.loadTransactionFilters(transaction.txIndex, signal)
        .then((keys) => {
          if (inspected !== transaction) return;

          setTraits(keys);
          if (point !== null) place(point);
        })
        .catch((error) => {
          if (signal.aborted || inspected !== transaction) return;

          for (const { value } of traitFields) setField(value, "unavailable");
          console.error(error);
        })
        .finally(() => {
          if (traitsController === controller) traitsController = null;
        });
    }, eager ? 0 : TRAITS_DWELL_MS);
  }

  /**
   * @param {BlockPreviewTransaction | null} transaction
   * @param {BlockPreviewPointer | null} nextPoint
   * @param {boolean} eager
   */
  function inspect(transaction, nextPoint, eager) {
    if (transaction === null || nextPoint === null) {
      abortPending();
      inspected = null;
      point = null;
      element.hidden = true;
      return;
    }

    element.hidden = false;
    point = nextPoint;

    if (transaction === inspected) {
      place(nextPoint);
      return;
    }

    abortPending();
    inspected = transaction;
    invalidateReadoutSize();

    setField(tx, `#${formatNumber(transaction.txIndex)}`);
    setField(fee, `${formatFeeRate(transaction.feeRate)} sat/vB`);
    setField(weight, formatWeight(transaction.weight));
    loadTraits(transaction, eager);

    const cached = txidLoader.load(
      transaction,
      eager,
      (loadedTxid) => {
        if (inspected !== transaction) return;

        invalidateReadoutSize();
        setField(txid, loadedTxid);
        txid.title = loadedTxid;
        if (point !== null) place(point);
      },
      (error, signal) => {
        if (signal.aborted) return;

        console.error(error);
      },
    );

    if (!cached) {
      txid.removeAttribute("title");
      setField(txid, "loading", true);
    }

    place(nextPoint);
  }

  return /** @type {const} */ ({
    destroy: abortPending,
    element,
    inspect,
  });
}

/** @typedef {import("./data.js").BlockPreviewTransaction} BlockPreviewTransaction */
/** @typedef {import("./filters/data.js").BlockPreviewFilterData} BlockPreviewFilterData */

/**
 * @typedef {Object} BlockPreviewPointer
 * @property {number} clientX
 * @property {number} clientY
 */

/**
 * @typedef {Object} BlockPreviewReadoutSize
 * @property {number} width
 * @property {number} height
 */
