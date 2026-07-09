import { onPlainClick } from "./events.js";

/**
 * @param {"tip"} name
 * @param {string} label
 * @param {string} mobileLabel
 * @param {string} title
 * @param {() => void} handler
 */
export function createEdgeButton(name, label, mobileLabel, title, handler) {
  const button = document.createElement("button");

  button.type = "button";
  button.title = title;
  button.ariaLabel = title;
  button.dataset.edge = name;
  button.dataset.mobileLabel = mobileLabel;
  button.textContent = label;
  onPlainClick(button, handler);

  return button;
}
