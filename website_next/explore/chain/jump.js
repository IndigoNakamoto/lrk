import { transitionMs } from "./transition.js";

/**
 * @param {HTMLElement} element
 * @param {() => void} onJump
 */
export function createJumpController(element, onJump) {
  /** @type {number | undefined} */
  let timeout;
  let jumping = false;

  /** @param {Event} [event] */
  function finish(event) {
    if (
      event instanceof TransitionEvent &&
      (event.target !== element || event.propertyName !== "opacity")
    ) {
      return;
    }

    onJump();
    cancel();
  }

  function cancel() {
    if (timeout !== undefined) {
      window.clearTimeout(timeout);
      timeout = undefined;
    }

    element.removeEventListener("transitionend", finish);
    delete element.dataset.jumping;
    jumping = false;
  }

  function jump() {
    if (jumping) return;

    jumping = true;
    element.dataset.jumping = "";
    element.addEventListener("transitionend", finish);
    timeout = window.setTimeout(
      finish,
      transitionMs(element, "opacity") + 50,
    );
  }

  return /** @type {const} */ ({
    cancel,
    jump,
  });
}
