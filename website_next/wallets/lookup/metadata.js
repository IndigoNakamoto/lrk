/**
 * @typedef {import("./stats.js").AddressStats} AddressStats
 * @typedef {import("./index.js").AddressClient} AddressClient
 */

/** @param {AddressClient} client */
export function createAddressMetadata(client) {
  /** @type {Map<string, Promise<AddressStats>>} */
  const cache = new Map();

  /** @param {string} address */
  function get(address) {
    let metadata = cache.get(address);

    if (!metadata) {
      metadata = client.getAddress(address, { cache: false });
      cache.set(address, metadata);
    }

    return metadata;
  }

  /** @param {readonly string[]} addresses */
  async function fetchAll(addresses) {
    await Promise.all(addresses.map(get));
  }

  return /** @type {const} */ ({
    fetchAll,
    get,
  });
}
