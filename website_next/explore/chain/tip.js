const TIP_BLOCK_THRESHOLD = 10;

/**
 * @param {Object} args
 * @param {HTMLButtonElement} args.button
 * @param {() => boolean} args.reachedTip
 * @param {() => number} args.newestHeight
 * @param {() => HTMLButtonElement | null} args.tipCube
 * @param {() => number | null} args.visibleConfirmedHeight
 * @param {() => boolean} args.hasVisibleProjected
 * @param {(element: Element) => boolean} args.isElementVisible
 */
export function createTipVisibility({
  button,
  reachedTip,
  newestHeight,
  tipCube,
  visibleConfirmedHeight,
  hasVisibleProjected,
  isElementVisible,
}) {
  let frame = 0;

  /** @param {boolean} visible */
  function setVisible(visible) {
    button.toggleAttribute("data-visible", visible);
    button.setAttribute("aria-hidden", String(!visible));
    button.tabIndex = visible ? 0 : -1;
  }

  function sync() {
    const cube = tipCube();
    const height = newestHeight();

    if (!reachedTip() || height < 0 || !cube) {
      setVisible(false);
      return;
    }

    const visibleHeight = visibleConfirmedHeight();
    if (hasVisibleProjected()) {
      setVisible(false);
      return;
    }

    setVisible(
      visibleHeight != null
        ? height - visibleHeight > TIP_BLOCK_THRESHOLD
        : !isElementVisible(cube),
    );
  }

  function schedule() {
    if (frame) return;

    frame = window.requestAnimationFrame(() => {
      frame = 0;
      sync();
    });
  }

  function cancel() {
    if (!frame) return;

    window.cancelAnimationFrame(frame);
    frame = 0;
  }

  setVisible(false);

  return /** @type {const} */ ({
    cancel,
    schedule,
    setVisible,
    sync,
  });
}
