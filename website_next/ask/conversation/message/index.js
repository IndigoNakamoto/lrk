import { renderMarkdown } from "../../markdown.js";

/** @typedef {"user" | "assistant"} MessageRole */

/**
 * @param {HTMLElement} content
 * @param {MessageRole} role
 * @param {string} text
 */
export function setMessageContent(content, role, text) {
  if (role === "assistant") renderMarkdown(content, text);
  else content.textContent = text;
}

/**
 * @param {MessageRole} role
 * @param {string} text
 */
export function createAskMessage(role, text) {
  const item = document.createElement("li");
  const label = document.createElement("strong");
  const content = document.createElement("div");

  item.dataset.role = role;
  label.append(role === "user" ? "You" : "Assistant");
  setMessageContent(content, role, text);
  item.append(label, content);

  return { item, content };
}
