/** @param {readonly string[]} addresses */
export function createBucketKey(addresses) {
  return [...addresses].sort().join("\n");
}
