import { mapConcurrent } from "../../concurrent.js";
import { createBucketKey } from "../bucket-key.js";

const HISTORY_CONCURRENCY = 4;
const MAX_SELECTED_ADDRESS_TXS = 100;
const CACHE_KEY_SEPARATOR = "\n\n";

const historyByBucketKey =
  /** @type {Map<string, Promise<Map<string, ApiTransaction[]>>>} */ (new Map());

/**
 * @typedef {import("../../scan/index.js").WalletAddress} WalletAddress
 * @typedef {import("./transaction.js").ApiTransaction} ApiTransaction
 */

/**
 * @typedef {Object} AddressHistoryClient
 * @property {(address: string, options?: { cache?: boolean }) => Promise<ApiTransaction[]>} getAddressTxs
 */

/**
 * @param {AddressHistoryClient} client
 * @param {readonly string[]} bucketAddresses
 * @param {ReadonlySet<string>} selectedAddresses
 * @returns {Promise<Map<string, ApiTransaction[]>>}
 */
async function fetchBucketHistory(client, bucketAddresses, selectedAddresses) {
  const history = /** @type {Map<string, ApiTransaction[]>} */ (new Map());

  await mapConcurrent(
    bucketAddresses,
    HISTORY_CONCURRENCY,
    async (address) => {
      const transactions = await client.getAddressTxs(address, { cache: false });

      if (selectedAddresses.has(address)) {
        history.set(address, transactions);
      }
    },
  );

  return history;
}

/** @param {WalletAddress} address */
function canLoadHistory(address) {
  return (
    address.txCount <= MAX_SELECTED_ADDRESS_TXS &&
    address.historyAddresses.length > 0
  );
}

/** @param {readonly WalletAddress[]} addresses */
function createHistoryCacheKey(addresses) {
  return [
    createBucketKey(addresses[0].historyAddresses),
    createBucketKey(addresses.map((address) => address.address)),
  ].join(CACHE_KEY_SEPARATOR);
}

/**
 * @param {AddressHistoryClient} client
 * @param {readonly WalletAddress[]} addresses
 * @returns {Promise<Map<string, ApiTransaction[]>>}
 */
async function loadBucket(client, addresses) {
  const selectedAddresses = addresses.filter(canLoadHistory);
  if (!selectedAddresses.length) return new Map();

  const key = createHistoryCacheKey(selectedAddresses);
  let history = historyByBucketKey.get(key);

  if (!history) {
    const bucketAddresses = selectedAddresses[0].historyAddresses;
    const selectedAddressSet = new Set(
      selectedAddresses.map((address) => address.address),
    );

    history = fetchBucketHistory(
      client,
      bucketAddresses,
      selectedAddressSet,
    ).catch((error) => {
      historyByBucketKey.delete(key);
      throw error;
    });
    historyByBucketKey.set(key, history);
  }

  return history;
}

export const addressHistory = /** @type {const} */ ({
  loadBucket,
});
