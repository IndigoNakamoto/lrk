/**
 * @param {(HTMLElement | null)[]} items
 * @param {HTMLElement} menu
 */
export function createSeriesHighlight(items, menu) {
  const seriesNodes = /** @type {SeriesNode[]} */ (items.map(() => []));
  const noSeries = -1;
  let previewedSeries = noSeries;

  /** @param {number} index */
  function scrollToItem(index) {
    const item = items[index];
    if (!item) return;

    const margin = Number.parseFloat(getComputedStyle(menu).paddingLeft);
    const itemRect = item.getBoundingClientRect();
    const menuRect = menu.getBoundingClientRect();

    if (itemRect.left < menuRect.left + margin) {
      menu.scrollBy({
        left: itemRect.left - menuRect.left - margin,
        behavior: "smooth",
      });
    } else if (itemRect.right > menuRect.right - margin) {
      menu.scrollBy({
        left: itemRect.right - menuRect.right + margin,
        behavior: "smooth",
      });
    }
  }

  /** @param {number} index */
  function highlightSeries(index) {
    for (const [itemIndex, item] of items.entries()) {
      if (!item) continue;
      setActive(item, itemIndex === index);
    }

    seriesNodes.forEach((nodes, nodeIndex) => {
      for (const node of nodes) {
        setActive(node, nodeIndex === index);
      }
    });
  }

  function clearHighlight() {
    for (const item of items) {
      if (item) clearElementState(item);
    }

    for (const nodes of seriesNodes) {
      for (const node of nodes) clearElementState(node);
    }
  }

  function clearInteractionHighlight() {
    clearPreview();
    clearHighlight();
  }

  /** @param {number} index */
  function previewSeries(index) {
    if (index === previewedSeries) return;

    clearPreview();
    scrollToItem(index);
    const item = items[index];
    if (item) item.dataset.preview = "";
    for (const node of seriesNodes[index]) {
      node.dataset.preview = "";
      node.parentNode?.appendChild(node);
    }
    previewedSeries = index;
  }

  function clearPreview() {
    if (previewedSeries === noSeries) return;

    const item = items[previewedSeries];
    if (item) delete item.dataset.preview;
    for (const node of seriesNodes[previewedSeries]) {
      delete node.dataset.preview;
    }
    previewedSeries = noSeries;
  }

  items.forEach((item, index) => {
    if (!item) return;
    item.addEventListener("pointerenter", () => highlightSeries(index));
    item.addEventListener("pointerleave", clearInteractionHighlight);
    item.addEventListener("focus", () => highlightSeries(index));
    item.addEventListener("blur", clearInteractionHighlight);
  });

  /**
   * @param {SVGPathElement | SVGCircleElement} node
   * @param {number} index
   */
  function addNode(node, index) {
    seriesNodes[index].push(node);
  }

  function clearNodes() {
    clearInteractionHighlight();

    for (const nodes of seriesNodes) {
      nodes.length = 0;
    }
  }

  return {
    addNode,
    clearPreview,
    clearNodes,
    preview: previewSeries,
  };
}

/**
 * @param {HTMLElement | SVGElement} element
 * @param {boolean} active
 */
function setActive(element, active) {
  if (active) {
    element.dataset.active = "";
    delete element.dataset.muted;
  } else {
    delete element.dataset.active;
    element.dataset.muted = "";
  }
}

/** @param {HTMLElement | SVGElement} element */
function clearElementState(element) {
  delete element.dataset.active;
  delete element.dataset.muted;
  delete element.dataset.preview;
}

/** @typedef {(SVGPathElement | SVGCircleElement)[]} SeriesNode */
