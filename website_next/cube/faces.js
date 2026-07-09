export const CUBE_FACE_SPECS = /** @type {const} */ ([
  ["glass", "bottom"],
  ["glass", "rear-right"],
  ["glass", "rear-left"],
  ["liquid", "bottom"],
  ["liquid", "rear-right"],
  ["liquid", "rear-left"],
  ["liquid", "right"],
  ["liquid", "left"],
  ["liquid", "top"],
  ["glass", "right"],
  ["glass", "left"],
  ["glass", "top"],
]);

/**
 * @template {keyof HTMLElementTagNameMap} T
 * @param {T} tag
 * @param {string} role
 * @param {string} side
 */
export function createCubeFace(tag, role, side) {
  const element = document.createElement(tag);

  element.dataset.face = "";
  element.dataset.role = role;
  element.dataset.side = side;

  return element;
}
