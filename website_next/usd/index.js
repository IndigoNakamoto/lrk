const FRACTION_DIGITS = 2;

/**
 * @typedef {Object} UsdAmountOptions
 * @property {boolean} [signed]
 * @property {"positive" | "negative"} [tone]
 * @property {"title"} [size]
 *
 * @typedef {Object} UsdPart
 * @property {string} text
 * @property {boolean} muted
 */

/**
 * @param {UsdPart[]} parts
 * @param {string} text
 * @param {boolean} muted
 */
function pushPart(parts, text, muted) {
  const last = parts[parts.length - 1];

  if (last && last.muted === muted) {
    last.text += text;
    return;
  }

  parts.push({ text, muted });
}

/**
 * @param {number} value
 */
function formatInteger(value) {
  return value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/**
 * @param {number} dollars
 */
function splitUsd(dollars) {
  const cents = Math.round(Math.abs(dollars) * 100);

  return {
    cents,
    whole: Math.floor(cents / 100),
    fraction: String(cents % 100).padStart(FRACTION_DIGITS, "0"),
  };
}

/**
 * @param {number} dollars
 * @param {UsdAmountOptions} [options]
 */
export function getUsdParts(dollars, options = {}) {
  const parts = /** @type {UsdPart[]} */ ([]);
  const { cents, whole, fraction } = splitUsd(dollars);
  const lastFractionDigit = Math.max(...[...fraction].map((digit, index) => {
    return digit === "0" ? -1 : index;
  }));

  if (options.signed && dollars > 0 && cents > 0) pushPart(parts, "+", false);
  if (dollars < 0 && cents > 0) pushPart(parts, "-", false);

  pushPart(parts, "$", true);
  pushPart(parts, formatInteger(whole), false);

  if (lastFractionDigit === -1) {
    pushPart(parts, ".", true);
    pushPart(parts, fraction, true);
    return parts;
  }

  pushPart(parts, ".", false);
  for (let index = 0; index < fraction.length; index += 1) {
    pushPart(parts, fraction[index], index > lastFractionDigit);
  }

  return parts;
}

/**
 * @param {HTMLElement} element
 * @param {UsdAmountOptions} options
 */
function syncUsdOptions(element, options) {
  if (options.tone) {
    element.dataset.usdTone = options.tone;
  } else {
    delete element.dataset.usdTone;
  }

  if (options.size) {
    element.dataset.usdSize = options.size;
  } else {
    delete element.dataset.usdSize;
  }
}

/**
 * @param {HTMLElement} element
 * @param {number} dollars
 * @param {UsdAmountOptions} [options]
 */
export function renderUsdAmount(element, dollars, options = {}) {
  element.dataset.usdAmount = "";
  syncUsdOptions(element, options);
  element.replaceChildren(...getUsdParts(dollars, options).map((part) => {
    const span = document.createElement("span");

    if (part.muted) span.dataset.usdMuted = "";
    span.append(part.text);

    return span;
  }));
}

/**
 * @template {keyof HTMLElementTagNameMap} Tag
 * @param {Tag} tag
 * @param {number} dollars
 * @param {UsdAmountOptions} [options]
 */
export function createUsdAmount(tag, dollars, options = {}) {
  const element = document.createElement(tag);

  renderUsdAmount(element, dollars, options);

  return element;
}

/**
 * @param {number} dollars
 */
export function formatUsd(dollars) {
  return new Intl.NumberFormat("en-US", {
    currency: "USD",
    maximumFractionDigits: 0,
    style: "currency",
  }).format(dollars);
}
