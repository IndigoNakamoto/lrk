import { ASK_MODEL } from "./models.js";
import { renderMarkdown } from "./markdown.js";

/** @typedef {ReturnType<typeof createAskView>} AskView */
/** @typedef {import("./storage.js").ChatMeta} ChatMeta */

/**
 * @template {keyof HTMLElementTagNameMap} Tag
 * @param {Tag} tag
 * @param {string} part
 * @returns {HTMLElementTagNameMap[Tag]}
 */
function createPart(tag, part) {
  const element = document.createElement(tag);
  element.dataset.ask = part;
  return element;
}

function createNewChatButton() {
  const button = createPart("button", "new-chat");

  button.type = "button";
  button.ariaLabel = "New chat";
  button.title = "New chat";
  button.append("+");
  return button;
}

export function createAskView() {
  const main = createPart("main", "page");
  const backdrop = createPart("button", "backdrop");
  const sidebar = createPart("aside", "sidebar");

  main.dataset.page = "ask";
  main.dataset.state = "unloaded";
  backdrop.type = "button";
  backdrop.ariaLabel = "Close chats";
  sidebar.id = "ask-sidebar";

  const history = createPart("section", "history");
  const chats = createPart("ol", "chats");

  history.append(chats);

  sidebar.append(history);

  const workspace = createPart("section", "workspace");
  const topbar = createPart("header", "topbar");
  const topbarMain = createPart("div", "topbar-main");
  const newChatButton = createNewChatButton();
  const sidebarToggle = createPart("button", "sidebar-toggle");
  const sidebarToggleIcon = createPart("span", "sidebar-toggle-icon");

  sidebarToggle.type = "button";
  sidebarToggle.ariaLabel = "Show chats";
  sidebarToggle.title = "Show chats";
  sidebarToggleIcon.ariaHidden = "true";
  sidebarToggleIcon.append("→");
  sidebarToggle.append(sidebarToggleIcon);
  sidebarToggle.setAttribute("aria-controls", sidebar.id);
  sidebarToggle.setAttribute("aria-expanded", "false");
  topbarMain.append(newChatButton, sidebarToggle);
  topbar.append(topbarMain);

  const conversation = createPart("section", "conversation");
  const empty = createPart("div", "empty");
  const eyebrow = createPart("p", "eyebrow");
  const greeting = document.createElement("h2");
  const description = document.createElement("p");
  const suggestions = createPart("div", "suggestions");
  const transcript = createPart("ol", "transcript");

  eyebrow.append("Bitcoin · Private · On-device");
  greeting.append("Ask about Bitcoin");
  description.append(
    "Explore the protocol, network, and on-chain data—privately, in your browser.",
  );

  for (const [label, prompt] of [
    ["Explain a metric", "Explain this Bitcoin metric simply: "],
    ["Explore mining", "Explain Bitcoin mining and proof of work."],
    ["Understand fees", "Explain how Bitcoin transaction fees work."],
  ]) {
    const suggestion = createPart("button", "suggestion");
    const suggestionLabel = document.createElement("span");
    const suggestionArrow = document.createElement("span");

    suggestion.type = "button";
    suggestion.dataset.prompt = prompt;
    suggestionLabel.append(label);
    suggestionArrow.ariaHidden = "true";
    suggestionArrow.append("↗");
    suggestion.append(suggestionLabel, suggestionArrow);
    suggestions.append(suggestion);
  }

  empty.append(eyebrow, greeting, description, suggestions);
  transcript.ariaLive = "polite";
  transcript.hidden = true;
  conversation.append(empty, transcript);

  const dock = createPart("footer", "dock");
  const loader = createPart("section", "loader");
  const activity = createPart("p", "activity");
  const loadButton = createPart("button", "load");
  const loadLabel = document.createElement("span");
  const loadDetail = document.createElement("small");
  const progress = document.createElement("progress");

  activity.role = "status";
  activity.ariaLive = "polite";
  activity.append("Not loaded");
  activity.hidden = true;
  loadButton.type = "button";
  loadLabel.append("Load model");
  loadDetail.append(ASK_MODEL.size);
  loadButton.append(loadLabel, loadDetail);
  progress.max = 100;
  progress.hidden = true;
  loader.append(loadButton, progress);

  const form = createPart("form", "form");
  const fieldset = document.createElement("fieldset");
  const composer = createPart("div", "composer");
  const inputLabel = document.createElement("label");
  const inputLabelText = createPart("span", "visually-hidden");
  const input = document.createElement("textarea");
  const composerBar = createPart("div", "composer-bar");
  const composerHint = createPart("span", "composer-hint");
  const composerActions = createPart("div", "composer-actions");
  const askButton = createPart("button", "send");
  const stopButton = createPart("button", "stop");

  form.hidden = true;
  fieldset.disabled = true;
  inputLabelText.append("Message assistant");
  input.name = "question";
  input.rows = 1;
  input.maxLength = 4_000;
  input.placeholder = "Ask anything";
  input.required = true;
  inputLabel.append(inputLabelText, input);
  composerHint.append("Enter to send · Shift Enter for a new line");
  askButton.type = "submit";
  askButton.ariaLabel = "Send message";
  askButton.title = "Send message";
  askButton.append("↑");
  stopButton.type = "button";
  stopButton.ariaLabel = "Stop generating";
  stopButton.title = "Stop generating";
  stopButton.hidden = true;
  composerActions.append(askButton, stopButton);
  composerBar.append(composerHint, composerActions);
  composer.append(inputLabel, composerBar);
  fieldset.append(composer);
  form.append(fieldset);
  dock.append(loader, form, activity);

  workspace.append(topbar, conversation, dock);
  main.append(backdrop, sidebar, workspace);

  return {
    main,
    backdrop,
    chats,
    sidebarToggle,
    suggestions,
    empty,
    conversation,
    transcript,
    loader,
    loadButton,
    loadLabel,
    loadDetail,
    progress,
    form,
    fieldset,
    input,
    activity,
    askButton,
    stopButton,
    newChatButtons: [newChatButton],
  };
}

/**
 * @param {HTMLOListElement} transcript
 * @param {"user" | "assistant"} role
 * @param {string} text
 */
export function appendMessage(transcript, role, text) {
  const item = createPart("li", "message");
  const label = createPart("strong", "role");
  const content = createPart("div", "content");

  item.dataset.role = role;
  label.append(role === "user" ? "You" : "Assistant");
  setMessageContent(content, role, text);
  item.append(label, content);
  transcript.append(item);
  transcript.hidden = false;

  return { item, content };
}

/**
 * @param {HTMLElement} content
 * @param {"user" | "assistant"} role
 * @param {string} text
 */
export function setMessageContent(content, role, text) {
  if (role === "assistant") renderMarkdown(content, text);
  else content.textContent = text;
}

/** @param {AskView} view */
export function clearMessages(view) {
  view.transcript.replaceChildren();
  view.transcript.hidden = true;
  view.empty.hidden = false;
  delete view.main.dataset.hasMessages;
}

/**
 * @param {AskView} view
 * @param {import("./storage.js").StoredMessage[]} messages
 */
export function renderMessages(view, messages) {
  clearMessages(view);
  for (const message of messages) {
    appendMessage(view.transcript, message.role, message.content);
  }
}

/**
 * @param {AskView} view
 * @param {ChatMeta[]} chats
 * @param {string} activeChatId
 */
export function renderChats(view, chats, activeChatId) {
  const scrollTop = view.chats.scrollTop;
  const items = chats.filter((chat) => chat.messageCount > 0).map((chat) => {
    const item = createPart("li", "chat");
    const active = chat.id === activeChatId;
    const select = createPart("button", "chat-select");
    const remove = createPart("button", "chat-remove");

    item.toggleAttribute("data-active", active);
    select.type = "button";
    select.dataset.action = "select";
    select.dataset.chatId = chat.id;
    if (active) select.setAttribute("aria-current", "page");
    select.title = chat.title;
    select.textContent = chat.title;
    remove.type = "button";
    remove.dataset.action = "remove";
    remove.dataset.chatId = chat.id;
    remove.dataset.label = "×";
    remove.ariaLabel = `Hold to delete ${chat.title}`;
    remove.title = "Hold to delete chat";
    remove.textContent = "×";
    item.append(select, remove);
    return item;
  });

  view.chats.replaceChildren(...items);
  view.chats.scrollTop = scrollTop;
}

/**
 * @param {AskView} view
 * @param {boolean} open
 */
export function setSidebarOpen(view, open) {
  view.main.toggleAttribute("data-sidebar-open", open);
  view.sidebarToggle.setAttribute("aria-expanded", String(open));
}
