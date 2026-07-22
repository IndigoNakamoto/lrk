/**
 * @typedef {Object} AskLayoutPart
 * @property {HTMLElement} element
 *
 * @typedef {Object} AskSidebarPart
 * @property {HTMLButtonElement} backdrop
 * @property {HTMLElement} controls
 * @property {HTMLElement} element
 *
 * @typedef {Object} AskLayoutOptions
 * @property {AskSidebarPart} sidebar
 * @property {AskLayoutPart} conversation
 * @property {AskLayoutPart} composer
 * @property {AskLayoutPart} loader
 */

/** @param {AskLayoutOptions} options */
export function createAskLayout(options) {
  const main = document.createElement("main");
  const workspace = document.createElement("section");
  const header = document.createElement("header");
  const dock = document.createElement("footer");

  main.dataset.page = "ask";
  main.dataset.state = "unloaded";
  header.append(options.sidebar.controls);
  dock.append(options.composer.element, options.loader.element);
  workspace.append(header, options.conversation.element, dock);
  main.append(options.sidebar.backdrop, options.sidebar.element, workspace);

  return { main };
}
