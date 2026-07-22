import { createAskComposer } from "./composer/index.js";
import { prepareContext } from "./context.js";
import { createAskConversation } from "./conversation/index.js";
import { createAskHero } from "./hero/index.js";
import { createAskLayout } from "./layout/index.js";
import { createAskLoader } from "./loader/index.js";
import { AskModel } from "./model.js";
import { createAskSidebar } from "./sidebar/index.js";
import { askStorage } from "./storage.js";
import { createAskTools } from "./tools/index.js";
import { createResponseTimer } from "./timing.js";

const SIDEBAR_PREFERENCE_KEY = "bitview.ask.sidebar-collapsed.v1";

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

export function createAskPage() {
  const model = new AskModel();
  const assistant = createAskTools();
  let workspace = askStorage.load();
  let chat = workspace.activeChat;
  let loaded = false;
  let busy = false;
  let checking = false;
  let loading = false;
  let followingOutput = false;
  const sidebarPreference = localStorage.getItem(SIDEBAR_PREFERENCE_KEY);
  let sidebarCollapsed = sidebarPreference === null
    ? workspace.chats.length <= 1
    : sidebarPreference === "true";
  const mobileSidebar = window.matchMedia("(max-width: 48rem)");

  const hero = createAskHero({
    onPrompt(prompt) {
      composer.value = prompt;
      composer.focus();
    },
  });
  const conversation = createAskConversation({
    hero: hero.element,
    onScroll() {
      if (!busy) return;

      followingOutput = conversation.isNearBottom();
      if (!followingOutput) conversation.cancelScroll();
    },
  });
  const composer = createAskComposer({
    onSubmit: submitQuestion,
    onStop() {
      model.stop();
      assistant.stop();
    },
  });
  const loader = createAskLoader({
    onLoad() {
      void loadModel(false);
    },
  });
  const sidebar = createAskSidebar({
    onNew: openNewChat,
    onToggle: toggleSidebar,
    onSelect: selectChat,
    onRemove: removeChat,
  });
  const { main } = createAskLayout({
    sidebar,
    conversation,
    composer,
    loader,
  });

  /** @param {boolean} open */
  function setSidebarOpen(open) {
    main.toggleAttribute("data-sidebar-open", open);
  }

  function syncControls() {
    sidebar.setBusy(busy);
    hero.setEnabled(loaded && !busy);
  }

  function syncSidebar() {
    const drawerOpen = main.hasAttribute("data-sidebar-open");
    const expanded = mobileSidebar.matches ? drawerOpen : !sidebarCollapsed;

    main.toggleAttribute("data-sidebar-collapsed", sidebarCollapsed);
    sidebar.setExpanded(expanded);
  }

  function renderChatList() {
    sidebar.render(workspace.chats, chat.id);
  }

  function renderChat() {
    renderChatList();
    conversation.render(chat.messages);
    syncControls();
    syncSidebar();
    conversation.scrollToBottom();
  }

  function setChecking() {
    loaded = false;
    busy = false;
    checking = true;
    loading = false;
    main.dataset.state = "checking";
    composer.hide();
    loader.checking();
    syncControls();
  }

  /** @param {string | undefined} [detail] */
  function setUnloaded(detail) {
    loaded = false;
    busy = false;
    checking = false;
    loading = false;
    main.dataset.state = "unloaded";
    composer.hide();
    loader.unloaded(detail);
    syncControls();
  }

  /** @param {boolean} cached */
  function setLoading(cached) {
    loaded = false;
    busy = false;
    checking = false;
    loading = true;
    main.dataset.state = "loading";
    composer.hide();
    loader.loading(cached);
    syncControls();
  }

  /** @param {unknown} error */
  function setLoadError(error) {
    loaded = false;
    busy = false;
    checking = false;
    loading = false;
    main.dataset.state = "error";
    composer.hide();
    loader.loadError(errorMessage(error));
    syncControls();
  }

  function setReady() {
    loaded = true;
    busy = false;
    checking = false;
    loading = false;
    main.dataset.state = "ready";
    loader.ready();
    composer.ready();
    syncControls();
  }

  /** @param {boolean} cached */
  async function loadModel(cached) {
    if (loaded || loading) return;

    setLoading(cached);

    try {
      await model.load(
        (update) => loader.reportProgress(update, cached),
        (status) => loader.reportStatus(status, cached),
      );
      setReady();
      composer.focus();
    } catch (error) {
      if (main.inert) return;

      model.terminate();
      setLoadError(error);
    }
  }

  async function activateModel() {
    if (loaded || loading || checking) return;

    setChecking();
    try {
      const modelCached = await model.isCached();
      if (main.inert) return;

      if (modelCached) await loadModel(true);
      else setUnloaded();
    } catch {
      if (main.inert) return;

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
      composer.value = "";
      renderChat();
    }

    setSidebarOpen(false);
    syncSidebar();
    if (loaded) {
      setReady();
      composer.focus();
    }
  }

  /** @param {string} id */
  function selectChat(id) {
    if (busy || id === chat.id) {
      setSidebarOpen(false);
      syncSidebar();
      return;
    }

    workspace = askStorage.select(id);
    chat = workspace.activeChat;
    model.reset();
    composer.value = "";
    renderChat();
    setSidebarOpen(false);
    syncSidebar();

    if (loaded) {
      setReady();
      composer.focus();
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
    composer.value = "";
    renderChat();

    if (loaded) setReady();
    else if (!checking && !loading) setUnloaded();
  }

  function toggleSidebar() {
    if (mobileSidebar.matches) {
      setSidebarOpen(!main.hasAttribute("data-sidebar-open"));
    } else {
      sidebarCollapsed = !sidebarCollapsed;
      localStorage.setItem(SIDEBAR_PREFERENCE_KEY, String(sidebarCollapsed));
      setSidebarOpen(false);
    }
    syncSidebar();
  }

  /** @param {string} question */
  async function submitQuestion(question) {
    if (!loaded || busy) return;

    composer.value = "";
    const questionMessage = conversation.append("user", question);
    const answerMessage = conversation.append("assistant", "");
    const timer = createResponseTimer(answerMessage.setSteps);
    const draft = {
      ...chat,
      messages: [
        ...chat.messages,
        { role: /** @type {const} */ ("user"), content: question },
      ],
    };

    busy = true;
    followingOutput = true;
    main.dataset.state = "generating";
    loader.ready();
    composer.generating();
    syncControls();
    conversation.scrollToBottom();

    try {
      timer.set("Preparing context");
      const prepared = await prepareContext(
        draft,
        model,
        assistant.toolsFor(),
        () => timer.set("Compacting conversation"),
      );
      timer.set("Routing request");
      const { output, artifacts } = await assistant.answer({
        chatId: chat.id,
        history: prepared.chat.messages,
        model,
        messages: prepared.messages,
        onToken({ text }) {
          answerMessage.content.append(text);
          if (followingOutput) conversation.scrollToBottom();
        },
        onStatus(status) {
          timer.set(status);
          if (followingOutput) conversation.scrollToBottom();
        },
      });

      const { elapsedMs, steps } = timer.finish();
      conversation.setContent(answerMessage.content, "assistant", output);
      answerMessage.setArtifacts(artifacts);
      answerMessage.setElapsed(elapsedMs);
      if (followingOutput) conversation.scrollToBottom();
      workspace = askStorage.save({
        ...prepared.chat,
        messages: [
          ...prepared.chat.messages,
          {
            role: /** @type {const} */ ("assistant"),
            content: output,
            elapsedMs,
            steps,
            ...(artifacts.length ? { artifacts } : {}),
          },
        ],
      });
      chat = workspace.activeChat;
      renderChatList();
      setReady();
      composer.focus();
    } catch (error) {
      questionMessage.item.remove();
      answerMessage.item.remove();
      composer.value = question;
      conversation.sync();

      if (main.inert) {
        setUnloaded();
        return;
      }

      setReady();
      loader.answerError(errorMessage(error));
      composer.focus();
    }
  }

  sidebar.backdrop.addEventListener("click", () => {
    setSidebarOpen(false);
    syncSidebar();
  });
  main.addEventListener("keydown", (event) => {
    if (event.key !== "Escape") return;

    setSidebarOpen(false);
    syncSidebar();
  });
  mobileSidebar.addEventListener("change", syncSidebar);
  main.addEventListener("pageactive", () => void activateModel());
  main.addEventListener("pageinactive", () => {
    model.terminate();
    assistant.terminate();
    setSidebarOpen(false);
    setUnloaded();
    syncSidebar();
  });

  renderChat();
  setUnloaded();
  return main;
}
