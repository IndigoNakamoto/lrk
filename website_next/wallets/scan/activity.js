/** @param {{ received: number, sent: number, txCount: number }} address */
export function isUsedAddress(address) {
  return address.received > 0 || address.sent > 0 || address.txCount > 0;
}
