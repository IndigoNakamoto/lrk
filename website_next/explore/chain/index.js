import { brk } from "../../utils/client.js";
import { createConfirmedBlocks } from "./confirmed.js";
import { createEdgeButton } from "./edge.js";
import {
  distanceFromViewport,
  findVisibleConfirmedHeight,
  isHorizontalLayout,
  olderWheelDelta,
  preserveScrollPosition,
} from "./scroll.js";
import { createJumpController } from "./jump.js";
import { createOlderBlocks } from "./older.js";
import { createProjectedBlocks } from "./projected.js";
import { createTipVisibility } from "./tip.js";

const BLOCK_BATCH_SIZE = 15;
const EDGE_LOAD_DISTANCE = 50;
const POLL_INTERVAL = 1_000;

/** @typedef {Awaited<ReturnType<typeof brk.getBlocksV1>>[number]} Block */
/** @typedef {Awaited<ReturnType<typeof brk.getMempoolBlocks>>[number]} MempoolBlock */

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
    const tipCube = confirmed.tipCube();
    if (tipCube) confirmed.select(tipCube, { scroll: "instant" });
  });

  element.id = "chain";
  scrollElement.dataset.chainScroll = "";
  blocksElement.dataset.chainBlocks = "";
  scrollElement.append(blocksElement);
  element.append(tipButton, scrollElement);

  const projected = createProjectedBlocks({
    blocksElement,
    isLayoutFrozen: () => tipButton.hasAttribute("data-visible"),
    isElementVisible,
  });
  const confirmed = createConfirmedBlocks({
    blocksElement,
    firstProjectedElement: projected.firstElement,
    onSelect,
    onScrollSelect: () => tip.schedule(),
  });
  const tip = createTipVisibility({
    button: tipButton,
    reachedTip: () => reachedTip,
    newestHeight: () => newestHeight,
    tipCube: confirmed.tipCube,
    visibleConfirmedHeight: () =>
      findVisibleConfirmedHeight(scrollElement, blocksElement),
    hasVisibleProjected: () => projected.hasVisibleElement(),
    isElementVisible,
  });

  let active = false;
  let newestHeight = -1;
  let newestTimestamp = 0;
  let loadingNewer = false;
  let polling = false;
  let reachedTip = false;

  /** @type {number | undefined} */
  let pollId;

  /** @type {AbortController} */
  let controller = new AbortController();

  const older = createOlderBlocks({
    scrollElement,
    blocksElement,
    batchSize: BLOCK_BATCH_SIZE,
    isActive: () => active,
    isHorizontal,
    fetchBlocks: (startHeight) =>
      brk.getBlocksV1FromHeight(startHeight, { signal: controller.signal }),
    createCube: confirmed.create,
    isAborted: () => controller.signal.aborted,
    onError: (error) => logChainError("explore older:", error),
  });

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
      return reachedTip && newestHeight >= 0 ? confirmed.newest() : null;
    }

    return confirmed.find(hashOrHeight);
  }

  function jumpToTip() {
    if (confirmed.tipCube()) jump.jump();
  }

  function isHorizontal() {
    return isHorizontalLayout(blocksElement);
  }

  function clear() {
    newestHeight = -1;
    newestTimestamp = 0;
    loadingNewer = false;
    reachedTip = false;
    confirmed.clear();
    blocksElement.textContent = "";
    projected.clear();
    older.reset();
    tip.setVisible(false);
  }

  /** @param {Block[]} blocks */
  function appendNewerBlocks(blocks) {
    if (!blocks.length) return false;

    const anchor = confirmed.newest();
    const anchorRect = anchor?.getBoundingClientRect();

    for (let i = blocks.length - 1; i >= 0; i--) {
      const block = blocks[i];

      if (block.height > newestHeight) {
        confirmed.append(block);
      } else {
        confirmed.cache(block);
      }
    }

    newestHeight = Math.max(newestHeight, blocks[0].height);
    newestTimestamp = blocks[0].timestamp;
    confirmed.markTip();
    refreshProjected();

    preserveScrollPosition(scrollElement, anchor, anchorRect);

    tip.sync();

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
      confirmed.prepend(block);
    }

    newestHeight = blocks[0].height;
    older.setOldestHeight(blocks[blocks.length - 1].height);
    newestTimestamp = blocks[0].timestamp;
    reachedTip = height == null;
    confirmed.markTip();
    older.reserve();

    if (reachedTip) await pollProjected();
    else await loadNewer();

    return blocks[0].id;
  }

  /** @param {string | number | null | undefined} hashOrHeight */
  async function resolveHeight(hashOrHeight) {
    if (typeof hashOrHeight === "number") return hashOrHeight;

    if (typeof hashOrHeight === "string") {
      const cached = confirmed.get(hashOrHeight);
      if (cached) return cached.height;

      const block = await brk.getBlockV1(hashOrHeight, {
        signal: controller.signal,
      });
      confirmed.cache(block);

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
      confirmed.select(existing, { scroll: "smooth" });
      return;
    }

    confirmed.markSkeletons();
    element.dataset.loading = "";

    try {
      const height = await resolveHeight(hashOrHeight);
      const startHash = await loadInitial(height);
      const cube = findCube(startHash);
      if (cube) confirmed.select(cube, { scroll: "instant" });
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

  /** @param {MempoolBlock[]} blocks */
  function renderProjected(blocks) {
    projected.render(blocks);
    confirmed.markTip();
    refreshProjected();
  }

  function refreshProjected() {
    projected.refresh(newestTimestamp);
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
    const cube = confirmed.newest();

    return cube != null && cubeDistanceFromViewport(cube) <= EDGE_LOAD_DISTANCE;
  }

  scrollElement.addEventListener(
    "wheel",
    (event) => {
      older.reserve(olderWheelDelta(event, isHorizontal()));
    },
    { passive: true },
  );

  scrollElement.addEventListener(
    "scroll",
    () => {
      tip.schedule();
      older.reserve();

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

    tip.cancel();
    jump.cancel();
    controller.abort();
  }

  return /** @type {const} */ ({
    element,
    setActive,
  });
}
