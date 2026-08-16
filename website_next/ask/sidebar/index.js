import { bindHold } from "../../utils/hold.js";

const SIDEBAR_ID = "ask-sidebar";

/** @typedef {import("../storage.js").ChatMeta} ChatMeta */

/**
 * @typedef {Object} AskSidebarOptions
 * @property {() => void} onNew
 * @property {() => Promise<void>} onCopy
 * @property {() => void} onToggle
 * @property {(id: string) => void} onSelect
 * @property {(id: string) => void} onRemove
 */

/**
 * @param {string} label
 * @param {string} glyph
 */
function createControl(label, glyph) {
  const button = document.createElement("button");
  const icon = document.createElement("span");

  button.type = "button";
  button.ariaLabel = label;
  button.title = label;
  icon.ariaHidden = "true";
  icon.append(glyph);
  button.append(icon);
  return button;
}

/** @param {AskSidebarOptions} options */
export function createAskSidebar(options) {
  const backdrop = document.createElement("button");
  const aside = document.createElement("aside");
  const chats = document.createElement("ol");
  const controls = document.createElement("nav");
  const newChat = createControl("New chat", "+");
  const toggle = createControl("Show chats", "→");
  let busy = false;
  let copying = false;
  let copyFeedback = 0;

  backdrop.type = "button";
  backdrop.ariaLabel = "Close chats";
  aside.id = SIDEBAR_ID;
  chats.setAttribute("aria-label", "Chats");
  controls.setAttribute("aria-label", "Chat controls");
  toggle.setAttribute("aria-controls", SIDEBAR_ID);
  toggle.setAttribute("aria-expanded", "false");
  newChat.addEventListener("click", options.onNew);
  toggle.addEventListener("click", options.onToggle);
  controls.append(newChat, toggle);
  aside.append(chats);

  /** @param {HTMLButtonElement} button @param {string} label @param {string} glyph */
  function showCopyFeedback(button, label, glyph) {
    const icon = /** @type {HTMLElement} */ (button.firstElementChild);

    clearTimeout(copyFeedback);
    button.ariaLabel = label;
    button.title = label;
    icon.textContent = glyph;
    copyFeedback = setTimeout(() => {
      button.ariaLabel = "Copy conversation";
      button.title = "Copy conversation";
      icon.textContent = "↗";
    }, 1_500);
  }

  function syncDisabled() {
    newChat.disabled = busy;
    toggle.disabled = busy;
    for (const button of chats.querySelectorAll("button")) {
      button.disabled = busy || copying;
    }
  }

  return {
    backdrop,
    controls,
    element: aside,
    /** @param {boolean} value */
    setBusy(value) {
      busy = value;
      syncDisabled();
    },
    /** @param {boolean} expanded */
    setExpanded(expanded) {
      const label = expanded ? "Hide chats" : "Show chats";

      toggle.setAttribute("aria-expanded", String(expanded));
      toggle.ariaLabel = label;
      toggle.title = label;
    },
    /**
     * @param {ChatMeta[]} items
     * @param {string} activeId
     */
    render(items, activeId) {
      const scrollTop = chats.scrollTop;
      const rows = items.filter((chat) => chat.messageCount > 0).map((chat) => {
        const item = document.createElement("li");
        const select = document.createElement("button");
        const remove = document.createElement("button");

        select.type = "button";
        select.title = chat.title;
        select.textContent = chat.title;
        if (chat.id === activeId) select.setAttribute("aria-current", "page");
        select.addEventListener("click", () => options.onSelect(chat.id));

        if (chat.id === activeId) {
          const copy = createControl("Copy conversation", "↗");

          copy.addEventListener("click", async () => {
            if (busy || copying) return;

            copying = true;
            syncDisabled();
            try {
              await options.onCopy();
              showCopyFeedback(copy, "Conversation copied", "✓");
            } catch {
              showCopyFeedback(copy, "Could not copy conversation", "!");
            } finally {
              copying = false;
              syncDisabled();
            }
          });
          item.append(select, copy);
        } else {
          item.append(select);
        }

        remove.type = "button";
        remove.ariaLabel = `Hold to delete ${chat.title}`;
        remove.title = "Hold to delete chat";
        const icon = document.createElement("span");

        icon.ariaHidden = "true";
        icon.append("×");
        remove.append(icon);
        bindHold(remove, () => options.onRemove(chat.id), 1_000);

        item.append(remove);
        return item;
      });

      chats.replaceChildren(...rows);
      chats.scrollTop = scrollTop;
      syncDisabled();
    },
  };
}
