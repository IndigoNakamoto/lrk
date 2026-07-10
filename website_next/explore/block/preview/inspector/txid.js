import { loadBlockPreviewTxid } from "../data.js";

const TXID_CACHE_LIMIT = 64;
const TXID_DWELL_MS = 120;

/**
 * @param {number} txIndex
 * @param {string} txid
 * @param {Map<number, string>} cache
 */
function rememberTxid(txIndex, txid, cache) {
  if (cache.has(txIndex)) cache.delete(txIndex);
  else if (cache.size >= TXID_CACHE_LIMIT) {
    cache.delete(/** @type {number} */ (cache.keys().next().value));
  }

  cache.set(txIndex, txid);
}

/**
 * @param {AbortSignal} parentSignal
 */
export function createTxidLoader(parentSignal) {
  const cache = /** @type {Map<number, string>} */ (new Map());
  let controller = /** @type {AbortController | null} */ (null);
  let timer = 0;

  function abort() {
    clearTimeout(timer);
    controller?.abort();
    controller = null;
  }

  /**
   * @param {BlockPreviewTransaction} transaction
   * @param {boolean} eager
   * @param {(txid: string) => void} onTxid
   * @param {(error: unknown, signal: AbortSignal) => void} onError
   * @returns {boolean}
   */
  function load(transaction, eager, onTxid, onError) {
    const cached = cache.get(transaction.txIndex);

    abort();

    if (cached !== undefined) {
      onTxid(cached);
      return true;
    }

    const nextController = new AbortController();

    controller = nextController;
    timer = setTimeout(() => {
      const signal = AbortSignal.any([parentSignal, nextController.signal]);

      void loadBlockPreviewTxid(transaction.txIndex, signal)
        .then((txid) => {
          rememberTxid(transaction.txIndex, txid, cache);
          onTxid(txid);
        })
        .catch((error) => onError(error, signal));
    }, eager ? 0 : TXID_DWELL_MS);

    return false;
  }

  return /** @type {const} */ ({
    abort,
    load,
  });
}

/** @typedef {import("../data.js").BlockPreviewTransaction} BlockPreviewTransaction */
