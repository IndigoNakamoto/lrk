/** @param {unknown} value @returns {value is Record<string, unknown>} */
function isObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/** @param {unknown} left @param {unknown} right */
function equalValue(left, right) {
  return String(left).trim() === String(right).trim();
}

/**
 * Some APIs return a page containing the requested resource. When a unique
 * item carries the same source-derived parameter name and value, focus that
 * item before selecting response fields.
 *
 * @param {unknown} data
 * @param {Record<string, unknown>} [arguments_]
 */
export function focusApiData(data, arguments_ = {}) {
  const page = isObject(data) &&
      typeof data.count === "number" &&
      Array.isArray(data.sample)
    ? data.sample
    : data;
  if (!Array.isArray(page)) return page;
  if (page.length === 1) return page[0];

  const supplied = Object.entries(arguments_);
  if (!supplied.length) return page;
  const matches = page.filter((item) =>
    isObject(item) &&
    supplied.every(([name, value]) =>
      Object.hasOwn(item, name) && equalValue(item[name], value)
    )
  );
  return matches.length === 1 ? matches[0] : page;
}
