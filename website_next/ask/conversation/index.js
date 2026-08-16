import {
  createAskMessage,
  setMessageContent,
} from "./message/index.js";

/** @typedef {import("../storage.js").StoredMessage} StoredMessage */

/**
 * @typedef {Object} AskConversationOptions
 * @property {HTMLElement} hero
 * @property {() => void} onScroll
 */

/** @param {AskConversationOptions} options */
export function createAskConversation(options) {
  const conversation = document.createElement("section");
  const transcript = document.createElement("ol");
  let scrollFrame = 0;

  conversation.dataset.askConversation = "";
  conversation.setAttribute("aria-label", "Conversation");
  transcript.ariaLive = "polite";
  transcript.hidden = true;
  conversation.append(options.hero, transcript);
  conversation.addEventListener("scroll", options.onScroll);

  function sync() {
    const hasMessages = transcript.childElementCount > 0;

    options.hero.hidden = hasMessages;
    transcript.hidden = !hasMessages;
  }

  return {
    element: conversation,
    sync,
    isNearBottom() {
      const { clientHeight, scrollHeight, scrollTop } = conversation;
      return scrollHeight - clientHeight - scrollTop < 96;
    },
    cancelScroll() {
      cancelAnimationFrame(scrollFrame);
    },
    scrollToBottom() {
      cancelAnimationFrame(scrollFrame);
      scrollFrame = requestAnimationFrame(() => {
        conversation.scrollTop = conversation.scrollHeight;
      });
    },
    /**
     * @param {"user" | "assistant"} role
     * @param {string} text
     * @param {import("../storage.js").StoredArtifact[]} [artifacts]
     */
    append(role, text, artifacts = []) {
      const message = createAskMessage(role, text, { artifacts });

      transcript.append(message.item);
      sync();
      return message;
    },
    /**
     * @param {HTMLElement} content
     * @param {"user" | "assistant"} role
     * @param {string} text
     */
    setContent: setMessageContent,
    /** @param {StoredMessage[]} messages */
    render(messages) {
      transcript.replaceChildren(
        ...messages.map((message) =>
          createAskMessage(
            message.role,
            message.content,
            {
              artifacts: message.artifacts,
              elapsedMs: message.elapsedMs,
              steps: message.steps,
            },
          ).item
        ),
      );
      sync();
    },
  };
}
