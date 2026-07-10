import { addressHistory } from "./address.js";
import { mapConcurrent } from "../../concurrent.js";
import { createBucketKey } from "../bucket-key.js";
import { readWalletTransaction } from "./transaction.js";

const HISTORY_BUCKET_CONCURRENCY = 2;

/**
 * @typedef {import("../../scan/index.js").WalletAddress} WalletAddress
 * @typedef {import("./transaction.js").ApiTransaction} ApiTransaction
 * @typedef {import("./transaction.js").WalletTransaction} WalletTransaction
 */

/**
 * @typedef {Object} TransactionClient
 * @property {(address: string, options?: { cache?: boolean }) => Promise<ApiTransaction[]>} getAddressTxs
 */

/**
 * @param {WalletAddress} address
 */
function isUsedAddress(address) {
  return address.txCount > 0;
}

/**
 * @param {readonly WalletAddress[]} addresses
 * @returns {WalletAddress[][]}
 */
function groupAddressesByBucket(addresses) {
  const groups = /** @type {Map<string, WalletAddress[]>} */ (new Map());

  for (const address of addresses) {
    const key = createBucketKey(address.historyAddresses);
    const group = groups.get(key) ?? [];

    group.push(address);
    groups.set(key, group);
  }

  return [...groups.values()];
}

/**
 * @param {WalletTransaction} a
 * @param {WalletTransaction} b
 */
function compareTransactions(a, b) {
  if (a.time === undefined && b.time === undefined) {
    return a.txid.localeCompare(b.txid);
  }

  if (a.time === undefined) return -1;
  if (b.time === undefined) return 1;

  return b.time - a.time;
}

/**
 * @param {TransactionClient} client
 * @param {readonly WalletAddress[]} addresses
 * @returns {Promise<WalletTransaction[]>}
 */
async function load(client, addresses) {
  const transactionsById = /** @type {Map<string, WalletTransaction>} */ (
    new Map()
  );
  const usedAddresses = addresses.filter(isUsedAddress);
  const histories = await mapConcurrent(
    groupAddressesByBucket(usedAddresses),
    HISTORY_BUCKET_CONCURRENCY,
    (group) => addressHistory.loadBucket(client, group),
  );

  for (const history of histories) {
    for (const transactions of history.values()) {
      for (const transaction of transactions) {
        const walletTransaction = readWalletTransaction(
          transaction,
          usedAddresses,
        );

        if (walletTransaction.txid) {
          transactionsById.set(walletTransaction.txid, walletTransaction);
        }
      }
    }
  }

  return [...transactionsById.values()].sort(compareTransactions);
}

export const historyCache = /** @type {const} */ ({
  load,
});
