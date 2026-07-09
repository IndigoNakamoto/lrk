const MONTHS = /** @type {const} */ ([
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
]);

/** @param {string} text */
export function dim(text) {
  const element = document.createElement("span");

  element.dataset.dim = "";
  element.textContent = text;

  return element;
}

/** @param {number} height */
export function createHeightElement(height) {
  const container = document.createElement("span");
  const prefix = document.createElement("span");
  const value = document.createElement("span");

  prefix.dataset.dim = "";
  prefix.textContent = `#${"0".repeat(Math.max(0, 7 - String(height).length))}`;
  value.textContent = String(height);
  container.append(prefix, value);

  return container;
}

/** @param {number} value */
export function formatNumber(value) {
  return value.toLocaleString();
}

/** @param {number} unixSeconds */
export function formatShortDate(unixSeconds) {
  const date = new Date(unixSeconds * 1_000);

  return `${MONTHS[date.getMonth()]} ${date.getDate()}`;
}

/** @param {number} unixSeconds */
export function formatHHMM(unixSeconds) {
  const date = new Date(unixSeconds * 1_000);

  return [
    String(date.getHours()).padStart(2, "0"),
    String(date.getMinutes()).padStart(2, "0"),
  ];
}
