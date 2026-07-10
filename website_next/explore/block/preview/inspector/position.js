const OVERLAY_MARGIN = 8;

/**
 * @param {HTMLElement} element
 * @param {BlockPreviewPointer} point
 * @param {BlockPreviewReadoutSize} size
 */
export function placeReadout(element, point, size) {
  const figure = /** @type {HTMLElement} */ (element.parentElement);
  const bounds = figure.getBoundingClientRect();
  const { width, height } = size;
  const minX = Math.min(bounds.width / 2, width / 2 + OVERLAY_MARGIN);
  const maxX = Math.max(minX, bounds.width - minX);
  const rawX = point.clientX - bounds.left;
  const rawY = point.clientY - bounds.top;
  const x = Math.min(maxX, Math.max(minX, rawX));
  const showBelow = rawY < height + OVERLAY_MARGIN * 2;
  const minY = showBelow ? OVERLAY_MARGIN : height + OVERLAY_MARGIN;
  const maxY = showBelow
    ? Math.max(minY, bounds.height - height - OVERLAY_MARGIN)
    : Math.max(minY, bounds.height - OVERLAY_MARGIN);
  const y = Math.min(maxY, Math.max(minY, rawY));

  element.style.setProperty("--tx-x", `${x}px`);
  element.style.setProperty("--tx-y", `${y}px`);
  element.toggleAttribute("data-below", showBelow);
}

/**
 * @typedef {Object} BlockPreviewPointer
 * @property {number} clientX
 * @property {number} clientY
 */

/**
 * @typedef {Object} BlockPreviewReadoutSize
 * @property {number} width
 * @property {number} height
 */
