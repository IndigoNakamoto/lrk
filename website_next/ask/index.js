import { prepareContext } from "./context.js";
import { AskModel } from "./model.js";
import { ASK_MODEL } from "./models.js";
import { askStorage } from "./storage.js";
import { bindHold } from "../utils/hold.js";
import {
  appendMessage,
  createAskView,
  renderChats,
  renderMessages,
  setMessageContent,
  setSidebarOpen,
} from "./view.js";

const SIDEBAR_PREFERENCE_KEY = "bitview.ask.sidebar-collapsed.v1";

/** @param {number} bytes */
function formatBytes(bytes) {
  return bytes >= 1_000_000
    ? `${(bytes / 1_000_000).toFixed(0)} MB`
    : `${(bytes / 1_000).toFixed(0)} KB`;
}

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

export function createAskPage() {
  const view = createAskView();
  const model = new AskModel();
  let workspace = askStorage.load();
  let chat = workspace.activeChat;
  let loaded = false;
  let busy = false;
  let checking = false;
  let loading = false;
  let followingOutput = false;
  let scrollFrame = 0;
  const sidebarPreference = localStorage.getItem(SIDEBAR_PREFERENCE_KEY);
  let sidebarCollapsed =
    sidebarPreference === null
      ? workspace.chats.length <= 1
      : sidebarPreference === "true";
  const mobileSidebar = window.matchMedia("(max-width: 48rem)");

  function isNearBottom() {
    const { clientHeight, scrollHeight, scrollTop } = view.conversation;
    return scrollHeight - clientHeight - scrollTop < 96;
  }

  function scrollToBottom() {
    cancelAnimationFrame(scrollFrame);
    scrollFrame = requestAnimationFrame(() => {
      view.conversation.scrollTop = view.conversation.scrollHeight;
    });
  }

  function syncConversation() {
    const hasMessages = view.transcript.childElementCount > 0;
    view.empty.hidden = hasMessages;
    view.transcript.hidden = !hasMessages;
  }

  function syncChatControls() {
    view.sidebarToggle.disabled = busy;

    for (const button of view.newChatButtons) {
      button.disabled = busy;
    }

    for (const button of view.suggestions.querySelectorAll("button")) {
      button.disabled = !loaded || busy;
    }

    for (const button of view.chats.querySelectorAll("button")) {
      button.disabled = busy;
    }
  }

  function syncSidebar() {
    const drawerOpen = view.main.hasAttribute("data-sidebar-open");
    const expanded = mobileSidebar.matches ? drawerOpen : !sidebarCollapsed;

    view.main.toggleAttribute("data-sidebar-collapsed", sidebarCollapsed);
    view.sidebarToggle.setAttribute("aria-expanded", String(expanded));
    view.sidebarToggle.ariaLabel = expanded ? "Hide chats" : "Show chats";
    view.sidebarToggle.title = view.sidebarToggle.ariaLabel;
  }

  function renderChatList() {
    renderChats(view, workspace.chats, chat.id);

    for (const button of view.chats.querySelectorAll(
      '[data-action="remove"]',
    )) {
      if (!(button instanceof HTMLButtonElement)) continue;

      bindHold(button, () => {
        const id = button.dataset.chatId;
        if (id) removeChat(id);
      }, 1_000);
    }
  }

  function renderChat() {
    renderChatList();
    renderMessages(view, chat.messages);
    syncConversation();
    syncChatControls();
    syncSidebar();
    scrollToBottom();
  }

  function setChecking() {
    loaded = false;
    busy = false;
    checking = true;
    loading = false;
    view.main.dataset.state = "checking";
    view.loader.hidden = false;
    view.form.hidden = true;
    view.fieldset.disabled = true;
    view.loadButton.hidden = true;
    view.progress.hidden = false;
    view.progress.removeAttribute("value");
    view.activity.hidden = false;
    view.activity.textContent = "Checking browser cache...";
    syncConversation();
    syncChatControls();
  }

  /** @param {string | undefined} [detail] */
  function setUnloaded(detail) {
    loaded = false;
    busy = false;
    checking = false;
    loading = false;
    view.main.dataset.state = "unloaded";
    view.loader.hidden = false;
    view.form.hidden = true;
    view.fieldset.disabled = true;
    view.loadButton.hidden = false;
    view.loadButton.disabled = false;
    view.loadLabel.textContent = "Download & use";
    view.loadDetail.textContent = ASK_MODEL.size;
    view.progress.hidden = true;
    view.activity.hidden = !detail;
    view.activity.textContent = detail ?? "";
    syncConversation();
    syncChatControls();
  }

  /** @param {boolean} cached */
  function setLoading(cached) {
    loaded = false;
    busy = false;
    checking = false;
    loading = true;
    view.main.dataset.state = "loading";
    view.loader.hidden = false;
    view.form.hidden = true;
    view.fieldset.disabled = true;
    view.loadButton.hidden = true;
    view.progress.hidden = false;
    if (cached) view.progress.removeAttribute("value");
    else view.progress.value = 0;
    view.activity.hidden = false;
    view.activity.textContent = cached
      ? "Loading the model from browser storage..."
      : "Preparing the one-time local download...";
    syncConversation();
    syncChatControls();
  }

  /** @param {unknown} error */
  function setLoadError(error) {
    loaded = false;
    busy = false;
    checking = false;
    loading = false;
    view.main.dataset.state = "error";
    view.loader.hidden = false;
    view.form.hidden = true;
    view.fieldset.disabled = true;
    view.progress.hidden = true;
    view.loadButton.hidden = false;
    view.loadButton.disabled = false;
    view.loadLabel.textContent = "Try again";
    view.activity.hidden = false;
    view.activity.textContent = `Load failed · ${errorMessage(error)}`;
    syncChatControls();
  }

  function setReady() {
    loaded = true;
    busy = false;
    checking = false;
    loading = false;
    view.main.dataset.state = "ready";
    view.loader.hidden = true;
    view.form.hidden = false;
    view.fieldset.disabled = false;
    view.input.disabled = false;
    view.askButton.hidden = false;
    view.stopButton.hidden = true;
    view.activity.hidden = true;
    syncConversation();
    syncChatControls();
  }

  /** @param {boolean} cached */
  async function loadModel(cached) {
    if (loaded || loading) return;

    setLoading(cached);

    try {
      await model.load(
        ({ progress, loaded: loadedBytes, total }) => {
          if (!cached) {
            view.progress.hidden = false;
            view.progress.value = progress;
          }
          view.activity.textContent = total
            ? cached
              ? "Loading the model from browser storage..."
              : `Downloading ${formatBytes(loadedBytes)} of ${formatBytes(total)}...`
            : cached
              ? "Starting the cached model..."
              : `Downloading model... ${progress.toFixed(0)}%`;
        },
        (status) => {
          view.activity.textContent =
            cached && status.includes("Downloading")
              ? "Loading the model from browser storage..."
              : status;
        },
      );

      setReady();
      view.input.focus();
    } catch (error) {
      if (view.main.inert) return;

      model.terminate();
      setLoadError(error);
    }
  }

  async function activateModel() {
    if (loaded || loading || checking) return;

    setChecking();
    try {
      const modelCached = await model.isCached();
      if (view.main.inert) return;

      if (modelCached) await loadModel(true);
      else setUnloaded();
    } catch {
      if (view.main.inert) return;

      model.terminate();
      setUnloaded("Cache status unavailable. Download the model to continue.");
    }
  }

  function openNewChat() {
    if (busy) return;

    if (chat.messages.length) {
      const wasSingleChat = workspace.chats.length === 1;
      workspace = askStorage.create();
      chat = workspace.activeChat;
      if (wasSingleChat) {
        sidebarCollapsed = false;
        localStorage.setItem(SIDEBAR_PREFERENCE_KEY, "false");
      }
      model.reset();
      view.input.value = "";
      renderChat();
    }

    setSidebarOpen(view, false);
    syncSidebar();
    if (loaded) view.input.focus();
  }

  /** @param {string} id */
  function selectChat(id) {
    if (busy || id === chat.id) {
      setSidebarOpen(view, false);
      syncSidebar();
      return;
    }

    workspace = askStorage.select(id);
    chat = workspace.activeChat;
    model.reset();
    view.input.value = "";
    renderChat();
    setSidebarOpen(view, false);
    syncSidebar();

    if (loaded) {
      setReady();
      view.input.focus();
    } else if (!checking && !loading) {
      setUnloaded();
    }
  }

  /** @param {string} id */
  function removeChat(id) {
    if (busy) return;

    const item = workspace.chats.find((candidate) => candidate.id === id);
    if (!item) return;

    workspace = askStorage.remove(id);
    chat = workspace.activeChat;
    model.reset();
    view.input.value = "";
    renderChat();

    if (loaded) setReady();
    else if (!checking && !loading) setUnloaded();
  }

  renderChat();

  view.loadButton.addEventListener("click", () => void loadModel(false));

  view.form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!loaded || busy) return;

    const question = view.input.value.trim();
    if (!question) return;

    view.input.value = "";
    const questionMessage = appendMessage(view.transcript, "user", question);
    const answerMessage = appendMessage(view.transcript, "assistant", "");
    const draft = {
      ...chat,
      messages: [
        ...chat.messages,
        { role: /** @type {const} */ ("user"), content: question },
      ],
    };
    busy = true;
    followingOutput = true;
    view.main.dataset.state = "generating";
    view.input.disabled = true;
    view.askButton.hidden = true;
    view.stopButton.hidden = false;
    view.activity.hidden = true;
    syncConversation();
    syncChatControls();
    scrollToBottom();

    try {
      const prepared = await prepareContext(draft, model, () => {});
      const output = await model.generate(
        prepared.messages,
        ({ text }) => {
          answerMessage.content.append(text);
          if (followingOutput) scrollToBottom();
        },
      );

      setMessageContent(answerMessage.content, "assistant", output);
      if (followingOutput) scrollToBottom();
      workspace = askStorage.save({
        ...prepared.chat,
        messages: [
          ...prepared.chat.messages,
          { role: /** @type {const} */ ("assistant"), content: output },
        ],
      });
      chat = workspace.activeChat;
      renderChatList();
      setReady();
      view.input.focus();
    } catch (error) {
      questionMessage.item.remove();
      answerMessage.item.remove();
      view.input.value = question;
      syncConversation();

      if (view.main.inert) {
        setUnloaded();
        return;
      }

      setReady();
      view.activity.hidden = false;
      view.activity.textContent = `Could not answer · ${errorMessage(error)}`;
      view.input.focus();
    }
  });

  view.stopButton.addEventListener("click", () => {
    model.stop();
  });

  view.conversation.addEventListener("scroll", () => {
    if (!busy) return;

    followingOutput = isNearBottom();
    if (!followingOutput) cancelAnimationFrame(scrollFrame);
  });

  for (const button of view.newChatButtons) {
    button.addEventListener("click", openNewChat);
  }

  view.sidebarToggle.addEventListener("click", () => {
    if (mobileSidebar.matches) {
      setSidebarOpen(view, !view.main.hasAttribute("data-sidebar-open"));
    } else {
      sidebarCollapsed = !sidebarCollapsed;
      localStorage.setItem(
        SIDEBAR_PREFERENCE_KEY,
        String(sidebarCollapsed),
      );
      setSidebarOpen(view, false);
    }
    syncSidebar();
  });
  view.backdrop.addEventListener("click", () => {
    setSidebarOpen(view, false);
    syncSidebar();
  });

  view.chats.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;

    const button = target.closest("button[data-chat-id]");
    if (!(button instanceof HTMLButtonElement)) return;
    const id = button.dataset.chatId;
    if (!id) return;
    if (button.dataset.action !== "select") return;

    selectChat(id);
  });

  view.suggestions.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;

    const button = target.closest("button[data-prompt]");
    if (!(button instanceof HTMLButtonElement)) return;

    view.input.value = button.dataset.prompt ?? "";
    view.input.focus();
  });

  view.input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    view.form.requestSubmit();
  });

  view.main.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;

    setSidebarOpen(view, false);
    syncSidebar();
  });

  mobileSidebar.addEventListener("change", syncSidebar);

  view.main.addEventListener("pageactive", () => void activateModel());

  view.main.addEventListener("pageinactive", () => {
    model.terminate();
    setSidebarOpen(view, false);
    setUnloaded();
    syncSidebar();
  });

  setUnloaded();
  return view.main;
}
