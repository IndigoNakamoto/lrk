import { isPlainLeftClick } from "../../utils/event.js";

/** @param {HTMLElement} element @param {() => void} handler */
export function onPlainClick(element, handler) {
  element.addEventListener("click", (event) => {
    if (!(event instanceof MouseEvent) || !isPlainLeftClick(event)) return;

    event.preventDefault();
    handler();
  });
}
