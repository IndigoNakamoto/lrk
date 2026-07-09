/**
 * @param {HTMLElement} parent
 * @param {string} group
 * @param {Node[]} children
 */
export function appendPane(parent, group, children) {
  if (!children.length) return;

  const section = document.createElement("section");

  section.dataset.group = group;
  section.append(...children);
  parent.append(section);
}
