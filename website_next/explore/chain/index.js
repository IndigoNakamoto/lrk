import { brk } from "../../utils/client.js";
import {
  createEnteringConfirmedCube,
  createPlaceholderCube,
  createProjectedCube,
  setConfirmedInterval,
  updateProjectedCube,
  updateProjectedTime,
} from "./block-cube.js";
import { createEdgeButton } from "./edge.js";
import {
  distanceFromViewport,
  findVisibleConfirmedHeight,
  isHorizontalLayout,
  olderRemaining,
  olderRunway,
  olderWheelDelta,
  preserveScrollPosition,
  scrollToElement,
} from "./scroll.js";
import { createJumpController } from "./jump.js";

const BLOCK_BATCH_SIZE = 15;
const EDGE_LOAD_DISTANCE = 50;
const OLDER_RESERVE_VIEWPORTS = 6;
const POLL_INTERVAL = 1_000;
const PROJECTED_LIMIT = 8;
const TARGET_BLOCK_SECONDS = 600;
const TIP_BLOCK_THRESHOLD = 10;

/** @typedef {Awaited<ReturnType<typeof brk.getBlocksV1>>[number]} Block */
/** @typedef {Awaited<ReturnType<typeof brk.getMempoolBlocks>>[number]} MempoolBlock */
/** @typedef {{ generation: number, startHeight: number, placeholders: HTMLElement[] }} OlderBatch */

/** @param {string | number | null | undefined} hashOrHeight */
function normalizeTarget(hashOrHeight) {
  if (hashOrHeight === "tip") return null;
  if (typeof hashOrHeight === "string" && /^\d+$/.test(hashOrHeight)) {
    return Number(hashOrHeight);
  }

  return hashOrHeight;
}

/**
 * @param {{ onSelect?: (block: Block) => void }} [options]
 */
export function createChain({ onSelect = () => {} } = {}) {
  const element = document.createElement("div");
  const scrollElement = document.createElement("div");
  const blocksElement = document.createElement("div");
  const tipButton = createEdgeButton("tip", "↑", "←", "Jump to chain tip", () => {
    jumpToTip();
  });
  const jump = createJumpController(element, () => {
    if (tipCube) selectCube(tipCube, { scroll: "instant" });
  });

  element.id = "chain";
  setTipVisible(false);
  scrollElement.dataset.chainScroll = "";
  blocksElement.dataset.chainBlocks = "";
  scrollElement.append(blocksElement);
  element.append(tipButton, scrollElement);

  /** @type {HTMLButtonElement | null} */
  let selectedCube = null;
  /** @type {HTMLButtonElement | null} */
  let tipCube = null;

  /** @type {Map<string, Block>} */
  const blocksByHash = new Map();

  /** @type {ReturnType<typeof createProjectedCube>[]} */
  const projectedCubes = [];

  let active = false;
  let newestHeight = -1;
  let oldestHeight = Infinity;
  let oldestReservedHeight = -1;
  let newestTimestamp = 0;
  let hydratingOlder = false;
  let loadingNewer = false;
  let polling = false;
  let reachedTip = false;
  let olderGeneration = 0;

  /** @type {OlderBatch[]} */
  const olderBatches = [];

  /** @type {number | undefined} */
  let pollId;
  let tipSyncFrame = 0;

  /** @type {AbortController} */
  let controller = new AbortController();

  /**
   * @param {string} label
   * @param {unknown} error
   */
  function logChainError(label, error) {
    if (!controller.signal.aborted) console.error(label, error);
  }

  /** @param {string | number | null | undefined} hashOrHeight */
  function findCube(hashOrHeight) {
    if (hashOrHeight == null) {
      return reachedTip && newestHeight >= 0 ? newestConfirmedCube() : null;
    }

    const attribute = typeof hashOrHeight === "number" ? "height" : "hash";

    return /** @type {HTMLButtonElement | null} */ (
      blocksElement.querySelector(`[data-${attribute}="${hashOrHeight}"]`)
    );
  }

  function firstProjectedCube() {
    return projectedCubes[0]?.element ?? null;
  }

  function newestConfirmedCube() {
    const firstProjected = firstProjectedCube();

    return /** @type {HTMLButtonElement | null} */ (
      firstProjected
        ? firstProjected.previousElementSibling
        : blocksElement.lastElementChild
    );
  }

  function deselectCube() {
    if (selectedCube) delete selectedCube.dataset.selected;
    selectedCube = null;
  }

  function updateTipCube() {
    tipCube?.removeAttribute("data-tip");
    tipCube = newestConfirmedCube();
    tipCube?.setAttribute("data-tip", "");
  }

  function jumpToTip() {
    if (tipCube) jump.jump();
  }

  /**
   * @param {HTMLButtonElement} cube
   * @param {{ scroll?: "smooth" | "instant" }} [options]
   */
  function selectCube(cube, { scroll } = {}) {
    if (cube !== selectedCube) {
      deselectCube();
      selectedCube = cube;
      cube.dataset.selected = "";
    }

    const hash = cube.dataset.hash;
    const block = hash ? blocksByHash.get(hash) : undefined;
    if (block) onSelect(block);

    if (scroll) {
      scrollToElement(cube, scroll);
      scheduleTipVisibilitySync();
    }
  }

  function markConfirmedSkeletons() {
    for (const cube of blocksElement.children) {
      if (!cube.hasAttribute("data-projected")) {
        cube.setAttribute("data-skeleton", "");
      }
    }
  }

  function isHorizontal() {
    return isHorizontalLayout(blocksElement);
  }

  /** @param {number} [delta] */
  function reserveOlderRunway(delta = 0) {
    if (!active || oldestReservedHeight <= 0) return;

    const horizontal = isHorizontal();
    const runway =
      olderRunway(scrollElement, horizontal, OLDER_RESERVE_VIEWPORTS) + delta;
    let remaining = olderRemaining(scrollElement, horizontal);

    while (remaining < runway) {
      if (!reserveOlderBatch()) return;
      remaining = olderRemaining(scrollElement, horizontal);
    }
  }

  function clear() {
    newestHeight = -1;
    oldestHeight = Infinity;
    oldestReservedHeight = -1;
    newestTimestamp = 0;
    hydratingOlder = false;
    loadingNewer = false;
    reachedTip = false;
    olderGeneration++;
    selectedCube = null;
    tipCube = null;
    blocksByHash.clear();
    blocksElement.textContent = "";
    projectedCubes.length = 0;
    olderBatches.length = 0;
    setTipVisible(false);
  }

  /**
   * @param {Element | null} anchor
   * @param {number} count
   */
  function prependOlderPlaceholders(anchor, count) {
    const fragment = document.createDocumentFragment();
    const placeholders = /** @type {HTMLElement[]} */ ([]);

    for (let i = 0; i < count; i++) {
      const cube = createPlaceholderCube();

      placeholders.push(cube);
      fragment.append(cube);
    }

    blocksElement.insertBefore(fragment, anchor);

    return placeholders;
  }

  function reserveOlderBatch() {
    if (!active || oldestReservedHeight <= 0) return false;

    const anchor = blocksElement.firstElementChild;
    const count = Math.min(BLOCK_BATCH_SIZE, oldestReservedHeight);
    const startHeight = oldestReservedHeight - 1;
    const placeholders = prependOlderPlaceholders(anchor, count);

    if (!placeholders.length) return false;

    oldestReservedHeight -= placeholders.length;
    olderBatches.push({ generation: olderGeneration, startHeight, placeholders });
    void hydrateOlderBatches();

    return true;
  }

  /** @param {Block} block */
  function createKnownEnteringConfirmedCube(block) {
    blocksByHash.set(block.id, block);

    return createEnteringConfirmedCube(block, selectCube);
  }

  /** @param {Block[]} blocks */
  function appendNewerBlocks(blocks) {
    if (!blocks.length) return false;

    const anchor = newestConfirmedCube();
    const anchorRect = anchor?.getBoundingClientRect();

    for (let i = blocks.length - 1; i >= 0; i--) {
      const block = blocks[i];

      if (block.height > newestHeight) {
        appendConfirmed(createKnownEnteringConfirmedCube(block));
      } else {
        blocksByHash.set(block.id, block);
      }
    }

    newestHeight = Math.max(newestHeight, blocks[0].height);
    newestTimestamp = blocks[0].timestamp;
    updateTipCube();
    refreshProjected();

    preserveScrollPosition(scrollElement, anchor, anchorRect);

    syncTipVisibility();

    return true;
  }

  /** @param {number | null} [height] */
  async function loadInitial(height) {
    const blocks =
      height != null
        ? await brk.getBlocksV1FromHeight(height, { signal: controller.signal })
        : await brk.getBlocksV1({ signal: controller.signal });

    clear();

    for (const block of blocks) {
      prependConfirmed(createKnownEnteringConfirmedCube(block));
    }

    newestHeight = blocks[0].height;
    oldestHeight = blocks[blocks.length - 1].height;
    oldestReservedHeight = oldestHeight;
    newestTimestamp = blocks[0].timestamp;
    reachedTip = height == null;
    updateTipCube();
    reserveOlderRunway();

    if (reachedTip) await pollProjected();
    else await loadNewer();

    return blocks[0].id;
  }

  /** @param {string | number | null | undefined} hashOrHeight */
  async function resolveHeight(hashOrHeight) {
    if (typeof hashOrHeight === "number") return hashOrHeight;

    if (typeof hashOrHeight === "string") {
      const cached = blocksByHash.get(hashOrHeight);
      if (cached) return cached.height;

      const block = await brk.getBlockV1(hashOrHeight, {
        signal: controller.signal,
      });
      blocksByHash.set(hashOrHeight, block);

      return block.height;
    }

    return null;
  }

  /** @param {string | number | null | undefined} [hashOrHeight] */
  async function goToCube(hashOrHeight) {
    if (!active) return;

    hashOrHeight = normalizeTarget(hashOrHeight);

    const existing = findCube(hashOrHeight);
    if (existing) {
      selectCube(existing, { scroll: "smooth" });
      return;
    }

    markConfirmedSkeletons();
    element.dataset.loading = "";

    try {
      const height = await resolveHeight(hashOrHeight);
      const startHash = await loadInitial(height);
      const cube = findCube(startHash);
      if (cube) selectCube(cube, { scroll: "instant" });
    } catch (error) {
      logChainError("explore chain load:", error);
    } finally {
      delete element.dataset.loading;
    }
  }

  async function pollProjected() {
    try {
      renderProjected(
        await brk.getMempoolBlocks({ signal: controller.signal }),
      );
    } catch (error) {
      logChainError("explore mempool:", error);
    }
  }

  async function poll() {
    if (!active || !reachedTip || polling) return;

    polling = true;
    await pollProjected();

    try {
      appendNewerBlocks(await brk.getBlocksV1({ signal: controller.signal }));
    } catch (error) {
      logChainError("explore chain poll:", error);
    } finally {
      polling = false;
    }
  }

  async function hydrateOlderBatches() {
    if (hydratingOlder) return;

    const generation = olderGeneration;

    hydratingOlder = true;

    try {
      while (
        active &&
        generation === olderGeneration &&
        olderBatches[0]?.generation === generation
      ) {
        await hydrateOlderBatch(olderBatches[0]);
        if (olderBatches[0]?.generation === generation) olderBatches.shift();
      }
    } finally {
      if (generation === olderGeneration) hydratingOlder = false;
    }
  }

  /** @param {OlderBatch} batch */
  async function hydrateOlderBatch(batch) {
    try {
      const blocks = await brk.getBlocksV1FromHeight(batch.startHeight, {
        signal: controller.signal,
      });

      if (!batch.placeholders.some((placeholder) => placeholder.isConnected)) {
        return;
      }

      const cubes = [...blocks].reverse().map(createKnownEnteringConfirmedCube);

      for (let i = 0; i < batch.placeholders.length; i++) {
        const cube = cubes[i];

        if (cube) batch.placeholders[i].replaceWith(cube);
        else batch.placeholders[i].remove();
      }

      for (const cube of cubes) setConfirmedInterval(cube);

      const next = cubes.at(-1)?.nextElementSibling;
      if (next instanceof HTMLElement) setConfirmedInterval(next);

      if (blocks.length) {
        oldestHeight = blocks[blocks.length - 1].height;
      } else {
        oldestReservedHeight = oldestHeight;
      }

      reserveOlderRunway();
    } catch (error) {
      if (controller.signal.aborted) return;

      for (const placeholder of batch.placeholders) placeholder.remove();
      oldestReservedHeight = oldestHeight;
      logChainError("explore older:", error);
    }
  }

  async function loadNewer() {
    if (!active || loadingNewer || newestHeight === -1 || reachedTip) return;

    loadingNewer = true;

    try {
      const prevNewest = newestHeight;
      const blocks = await brk.getBlocksV1FromHeight(
        newestHeight + BLOCK_BATCH_SIZE,
        { signal: controller.signal },
      );

      if (!appendNewerBlocks(blocks) || newestHeight === prevNewest) {
        reachedTip = true;
        await pollProjected();
      }
    } catch (error) {
      logChainError("explore newer:", error);
    } finally {
      loadingNewer = false;
    }
  }

  /** @param {HTMLButtonElement} cube */
  function prependConfirmed(cube) {
    const oldFirst = /** @type {HTMLElement | null} */ (
      blocksElement.firstElementChild
    );

    blocksElement.insertBefore(cube, oldFirst);
    if (oldFirst) setConfirmedInterval(oldFirst);
  }

  /** @param {HTMLButtonElement} cube */
  function appendConfirmed(cube) {
    blocksElement.insertBefore(cube, firstProjectedCube());
    setConfirmedInterval(cube);
  }

  /** @param {MempoolBlock[]} blocks */
  function renderProjected(blocks) {
    const want = Math.min(blocks.length, PROJECTED_LIMIT);

    while (projectedCubes.length > want) {
      projectedCubes.pop()?.element.remove();
    }

    while (projectedCubes.length < want) {
      const cube = createProjectedCube();
      projectedCubes.push(cube);
      blocksElement.append(cube.element);
    }

    for (let i = 0; i < want; i++) {
      updateProjectedCube(projectedCubes[i], blocks[i]);
    }

    updateTipCube();
    refreshProjected();
  }

  function refreshProjected() {
    if (!projectedCubes.length || !newestTimestamp) return;

    const now = Math.floor(Date.now() / 1_000);
    const elapsed = Math.max(0, now - newestTimestamp);
    const updateLayout = !tipButton.hasAttribute("data-visible");

    for (let i = 0; i < projectedCubes.length; i++) {
      const cube = projectedCubes[i];
      const interval = i === 0 ? elapsed : TARGET_BLOCK_SECONDS;
      const timestamp = now + i * TARGET_BLOCK_SECONDS;

      if (updateLayout) {
        cube.element.style.setProperty("--block-interval", String(interval));
      }

      updateProjectedTime(cube, timestamp);
    }
  }

  function scheduleTipVisibilitySync() {
    if (tipSyncFrame) return;

    tipSyncFrame = window.requestAnimationFrame(() => {
      tipSyncFrame = 0;
      syncTipVisibility();
    });
  }

  function cancelTipVisibilitySync() {
    if (!tipSyncFrame) return;

    window.cancelAnimationFrame(tipSyncFrame);
    tipSyncFrame = 0;
  }

  /** @param {boolean} visible */
  function setTipVisible(visible) {
    tipButton.toggleAttribute("data-visible", visible);
    tipButton.setAttribute("aria-hidden", String(!visible));
    tipButton.tabIndex = visible ? 0 : -1;
  }

  function syncTipVisibility() {
    if (!reachedTip || newestHeight < 0 || !tipCube) {
      setTipVisible(false);
      return;
    }

    const visibleHeight = findVisibleConfirmedHeight(scrollElement, blocksElement);
    if (projectedCubes.some(({ element }) => isElementVisible(element))) {
      setTipVisible(false);
      return;
    }

    setTipVisible(
      visibleHeight != null
        ? newestHeight - visibleHeight > TIP_BLOCK_THRESHOLD
        : !isElementVisible(tipCube),
    );
  }

  /** @param {Element} element */
  function cubeDistanceFromViewport(element) {
    return distanceFromViewport(scrollElement, element, isHorizontal());
  }

  /** @param {Element} element */
  function isElementVisible(element) {
    return cubeDistanceFromViewport(element) === 0;
  }

  function shouldLoadNewer() {
    const cube = newestConfirmedCube();

    return cube != null && cubeDistanceFromViewport(cube) <= EDGE_LOAD_DISTANCE;
  }

  scrollElement.addEventListener(
    "wheel",
    (event) => {
      reserveOlderRunway(olderWheelDelta(event, isHorizontal()));
    },
    { passive: true },
  );

  scrollElement.addEventListener(
    "scroll",
    () => {
      scheduleTipVisibilitySync();
      reserveOlderRunway();

      if (reachedTip || loadingNewer) return;
      if (shouldLoadNewer()) void loadNewer();
    },
    { passive: true },
  );

  /** @param {boolean} nextActive */
  function setActive(nextActive) {
    if (active === nextActive) return;

    active = nextActive;

    if (active) {
      controller = new AbortController();

      if (newestHeight === -1) void goToCube(null);
      else void poll();

      pollId = window.setInterval(() => void poll(), POLL_INTERVAL);
      return;
    }

    if (pollId !== undefined) {
      window.clearInterval(pollId);
      pollId = undefined;
    }

    cancelTipVisibilitySync();
    jump.cancel();
    controller.abort();
  }

  return /** @type {const} */ ({
    element,
    setActive,
  });
}
