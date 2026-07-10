import { createEnteringConfirmedCube, setConfirmedInterval } from "./block-cube.js";
import { scrollToElement } from "./scroll.js";

/** @typedef {import("../../modules/brk-client/index.js").BlockInfoV1} Block */

/**
 * @param {Object} args
 * @param {HTMLElement} args.blocksElement
 * @param {() => Element | null} args.firstProjectedElement
 * @param {(block: Block) => void} args.onSelect
 * @param {() => void} args.onScrollSelect
 */
export function createConfirmedBlocks({
  blocksElement,
  firstProjectedElement,
  onSelect,
  onScrollSelect,
}) {
  /** @type {HTMLButtonElement | null} */
  let selectedCube = null;
  /** @type {HTMLButtonElement | null} */
  let tipCube = null;
  /** @type {Map<string, Block>} */
  const blocksByHash = new Map();

  function clear() {
    selectedCube = null;
    tipCube = null;
    blocksByHash.clear();
  }

  /** @param {Block} block */
  function cache(block) {
    blocksByHash.set(block.id, block);
  }

  /** @param {string} hash */
  function get(hash) {
    return blocksByHash.get(hash);
  }

  /** @param {string | number} hashOrHeight */
  function find(hashOrHeight) {
    const attribute = typeof hashOrHeight === "number" ? "height" : "hash";

    return /** @type {HTMLButtonElement | null} */ (
      blocksElement.querySelector(`[data-${attribute}="${hashOrHeight}"]`)
    );
  }

  function newest() {
    const firstProjected = firstProjectedElement();

    return /** @type {HTMLButtonElement | null} */ (
      firstProjected
        ? firstProjected.previousElementSibling
        : blocksElement.lastElementChild
    );
  }

  function markTip() {
    tipCube?.removeAttribute("data-tip");
    tipCube = newest();
    tipCube?.setAttribute("data-tip", "");
  }

  function deselect() {
    if (selectedCube) delete selectedCube.dataset.selected;
    selectedCube = null;
  }

  /**
   * @param {HTMLButtonElement} cube
   * @param {{ scroll?: "smooth" | "instant" }} [options]
   */
  function select(cube, { scroll } = {}) {
    if (cube !== selectedCube) {
      deselect();
      selectedCube = cube;
      cube.dataset.selected = "";
    }

    const hash = cube.dataset.hash;
    const block = hash ? get(hash) : undefined;
    if (block) onSelect(block);

    if (scroll) {
      scrollToElement(cube, scroll);
      onScrollSelect();
    }
  }

  function markSkeletons() {
    for (const cube of blocksElement.children) {
      if (!cube.hasAttribute("data-projected")) {
        cube.setAttribute("data-skeleton", "");
      }
    }
  }

  /** @param {Block} block */
  function create(block) {
    cache(block);

    return createEnteringConfirmedCube(block, select);
  }

  /** @param {Block} block */
  function prepend(block) {
    const cube = create(block);
    const oldFirst = /** @type {HTMLElement | null} */ (
      blocksElement.firstElementChild
    );

    blocksElement.insertBefore(cube, oldFirst);
    if (oldFirst) setConfirmedInterval(oldFirst);

    return cube;
  }

  /** @param {Block} block */
  function append(block) {
    const cube = create(block);

    blocksElement.insertBefore(cube, firstProjectedElement());
    setConfirmedInterval(cube);

    return cube;
  }

  return /** @type {const} */ ({
    append,
    cache,
    clear,
    create,
    find,
    get,
    markSkeletons,
    markTip,
    newest,
    select,
    tipCube: () => tipCube,
    prepend,
  });
}
