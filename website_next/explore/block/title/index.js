/** @param {number} height */
function createBlockHeight(height) {
  const element = document.createElement("span");
  const prefix = document.createElement("span");
  const value = document.createElement("span");
  const text = String(height);

  prefix.dataset.dim = "";
  prefix.textContent = `#${"0".repeat(Math.max(0, 7 - text.length))}`;
  value.textContent = text;
  element.append(prefix, value);

  return element;
}

/** @param {number} height */
export function createBlockTitle(height) {
  const label = document.createElement("span");
  const value = document.createElement("span");

  label.dataset.titleLabel = "";
  value.dataset.titleHeight = "";
  label.textContent = "Block";
  value.append(createBlockHeight(height));

  return /** @type {const} */ ([label, value]);
}
