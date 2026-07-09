/**
 * @param {Object} args
 * @param {string} args.label
 * @param {string} args.color
 * @param {string} [args.ariaLabel]
 * @param {string | Node} [args.detail]
 */
export function createLegendItem(args) {
  const button = document.createElement("button");
  const label = document.createElement("span");
  const value = document.createElement("output");

  button.type = "button";
  button.dataset.legendItem = "";
  button.style.setProperty("--color", args.color);
  button.setAttribute("aria-label", args.ariaLabel ?? args.label);
  label.dataset.legendLabel = "";
  value.dataset.legendValue = "";
  label.append(args.label);
  button.append(label, value);

  if (args.detail != null) {
    const detail = document.createElement("output");

    detail.dataset.legendDetail = "";
    detail.append(args.detail);
    button.append(detail);
  }

  return { button, label, value };
}

/**
 * @param {Object} [args]
 * @param {boolean} [args.fill]
 * @param {boolean} [args.scroll]
 */
export function createLegendList(args = {}) {
  const list = document.createElement("menu");

  list.dataset.legendList = "";
  if (args.fill) list.dataset.legendFill = "";
  if (args.scroll) list.dataset.legendScroll = "";

  return list;
}

/**
 * @param {HTMLElement} list
 * @param {HTMLElement} item
 */
export function appendLegendListItem(list, item) {
  const row = document.createElement("li");

  row.append(item);
  list.append(row);

  return row;
}
