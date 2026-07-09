import { createCube } from "../cube/index.js";

/**
 * @param {string | undefined} href
 * @param {string} label
 */
function createBrandElement(href, label) {
  if (href) {
    const link = document.createElement("a");

    link.href = href;
    link.setAttribute("aria-label", `${label} home`);

    return link;
  }

  return document.createElement("p");
}

/**
 * @param {{
 *   href?: string,
 *   tone?: "default" | "receipt",
 *   animated?: boolean,
 *   fill?: number,
 * }} [options]
 */
export function createBrand({
  href,
  tone = "default",
  animated = Boolean(href),
  fill = tone === "receipt" ? 1 : 0.5,
} = {}) {
  const label = "litview";
  const brand = createBrandElement(href, label);
  const mark = document.createElement("span");
  const text = document.createElement("span");

  brand.dataset.brand = "";
  brand.dataset.tone = tone;
  if (animated) brand.dataset.animated = "";
  mark.dataset.brandMark = "";
  text.dataset.brandText = "";
  text.textContent = label;
  mark.append(createCube({ fill }));
  brand.append(mark, text);

  return brand;
}
