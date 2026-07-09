const PRESS_SELECTOR = [
  "a[href]",
  "button:not(:disabled)",
  "input:not(:disabled)",
  "label",
  "select:not(:disabled)",
  "summary",
  "textarea:not(:disabled)",
].join(",");

/** @type {Element | null} */
let pressedElement = null;

function clearPress() {
  pressedElement?.removeAttribute("data-press");
  pressedElement = null;
}

document.addEventListener(
  "pointerdown",
  (event) => {
    if (event.pointerType === "mouse") return;
    if (!(event.target instanceof Element)) return;

    clearPress();
    pressedElement = event.target.closest(PRESS_SELECTOR);
    pressedElement?.setAttribute("data-press", "");
  },
  { passive: true },
);

document.addEventListener("pointerup", clearPress, { passive: true });
document.addEventListener("pointercancel", clearPress, { passive: true });
window.addEventListener("blur", clearPress);
