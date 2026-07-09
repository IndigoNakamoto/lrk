/** @param {unknown} value */
export function readObject(value) {
  return value && typeof value === "object"
    ? /** @type {Record<string, unknown>} */ (value)
    : undefined;
}

/** @param {unknown} value */
export function readNumber(value) {
  return typeof value === "number" && Number.isFinite(value)
    ? value
    : undefined;
}

/** @param {unknown} value */
export function readString(value) {
  return typeof value === "string" ? value : undefined;
}

/**
 * @param {unknown} value
 * @param {string} key
 */
export function readArray(value, key) {
  const array = readObject(value)?.[key];

  return Array.isArray(array) ? array : [];
}
