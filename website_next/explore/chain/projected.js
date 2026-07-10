import {
  createProjectedCube,
  updateProjectedCube,
  updateProjectedTime,
} from "./block-cube.js";

const PROJECTED_LIMIT = 8;
const TARGET_BLOCK_SECONDS = 600;

/** @typedef {import("../../modules/brk-client/index.js").MempoolBlock} MempoolBlock */

/**
 * @param {Object} args
 * @param {HTMLElement} args.blocksElement
 * @param {() => boolean} args.isLayoutFrozen
 * @param {(element: Element) => boolean} args.isElementVisible
 */
export function createProjectedBlocks({
  blocksElement,
  isLayoutFrozen,
  isElementVisible,
}) {
  /** @type {ReturnType<typeof createProjectedCube>[]} */
  const cubes = [];

  function firstElement() {
    return cubes[0]?.element ?? null;
  }

  function clear() {
    cubes.length = 0;
  }

  /** @param {MempoolBlock[]} blocks */
  function render(blocks) {
    const want = Math.min(blocks.length, PROJECTED_LIMIT);

    while (cubes.length > want) {
      cubes.pop()?.element.remove();
    }

    while (cubes.length < want) {
      const cube = createProjectedCube();

      cubes.push(cube);
      blocksElement.append(cube.element);
    }

    for (let i = 0; i < want; i++) {
      updateProjectedCube(cubes[i], blocks[i]);
    }
  }

  /** @param {number} newestTimestamp */
  function refresh(newestTimestamp) {
    if (!cubes.length || !newestTimestamp) return;

    const now = Math.floor(Date.now() / 1_000);
    const elapsed = Math.max(0, now - newestTimestamp);
    const updateLayout = !isLayoutFrozen();

    for (let i = 0; i < cubes.length; i++) {
      const cube = cubes[i];
      const interval = i === 0 ? elapsed : TARGET_BLOCK_SECONDS;
      const timestamp = now + i * TARGET_BLOCK_SECONDS;

      if (updateLayout) {
        cube.element.style.setProperty("--block-interval", String(interval));
      }

      updateProjectedTime(cube, timestamp);
    }
  }

  function hasVisibleElement() {
    return cubes.some(({ element }) => isElementVisible(element));
  }

  return /** @type {const} */ ({
    clear,
    firstElement,
    hasVisibleElement,
    refresh,
    render,
  });
}
