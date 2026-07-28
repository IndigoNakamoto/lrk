import { createAskComposer } from "./composer/index.js";
import { createAskCompactor } from "./compactor.js";
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
import { conversationMarkdown } from "./transcript.js";

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
  let busy = false;
  /** @type {"unloaded" | "checking" | "loading" | "error" | "ready"} */
  let modelState = "unloaded";
  let followingOutput = false;
  const compactor = createAskCompactor({
    model,
    onCompacted(id, update) {
      const saved = askStorage.saveMemory(id, update);
      if (!saved || chat.id !== id) return;
      chat = saved;
      workspace = { ...workspace, activeChat: saved };
    },
  });
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
    async onCopy() {
      await navigator.clipboard.writeText(conversationMarkdown(chat));
    },
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

  /** @param {"unloaded" | "checking" | "loading" | "error" | "ready"} nextState */
  function setModelState(nextState) {
    modelState = nextState;
  }

  function syncControls() {
    sidebar.setBusy(busy);
    hero.setEnabled(modelState === "ready" && !busy);
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
    busy = false;
    setModelState("checking");
    composer.hide();
    loader.checking();
    syncControls();
  }

  /** @param {string | undefined} [detail] */
  function setUnloaded(detail) {
    busy = false;
    setModelState("unloaded");
    composer.hide();
    loader.unloaded(detail);
    syncControls();
  }

  /** @param {boolean} cached */
  function setLoading(cached) {
    busy = false;
    setModelState("loading");
    composer.hide();
    loader.loading(cached);
    syncControls();
  }

  /** @param {unknown} error */
  function setLoadError(error) {
    busy = false;
    setModelState("error");
    composer.hide();
    loader.loadError(errorMessage(error));
    syncControls();
  }

  function setReady() {
    busy = false;
    setModelState("ready");
    loader.ready();
    composer.ready();
    syncControls();
  }

  /** @param {boolean} cached */
  async function loadModel(cached) {
    if (modelState === "ready" || modelState === "loading") return;

    setLoading(cached);

    try {
      const modelLoading = model.load(
        (update) => loader.reportProgress(update, cached),
        (status) => loader.reportStatus(status, cached),
      );
      void assistant.prewarm().catch(() => {});
      await modelLoading;
      setReady();
      composer.focus();
    } catch (error) {
      if (main.inert) return;

      model.terminate();
      setLoadError(error);
    }
  }

  async function activateModel() {
    if (modelState === "ready" || modelState === "loading" || modelState === "checking") {
      return;
    }

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

  async function openNewChat() {
    if (busy) return;
    busy = true;
    syncControls();
    await compactor.cancel();
    if (main.inert) return;
    busy = false;

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
    if (modelState === "ready") {
      setReady();
      composer.focus();
    } else syncControls();
  }

  /** @param {string} id */
  async function selectChat(id) {
    if (busy || id === chat.id) {
      setSidebarOpen(false);
      syncSidebar();
      return;
    }
    busy = true;
    syncControls();
    await compactor.cancel();
    if (main.inert) return;
    busy = false;

    workspace = askStorage.select(id);
    chat = workspace.activeChat;
    model.reset();
    composer.value = "";
    renderChat();
    setSidebarOpen(false);
    syncSidebar();

    if (modelState === "ready") {
      setReady();
      composer.focus();
    } else if (modelState !== "checking" && modelState !== "loading") {
      setUnloaded();
    }
  }

  /** @param {string} id */
  async function removeChat(id) {
    if (busy) return;

    const item = workspace.chats.find((candidate) => candidate.id === id);
    if (!item) return;
    busy = true;
    syncControls();
    await compactor.cancel();
    if (main.inert) return;
    busy = false;

    workspace = askStorage.remove(id);
    chat = workspace.activeChat;
    model.reset();
    composer.value = "";
    renderChat();

    if (modelState === "ready") setReady();
    else if (modelState !== "checking" && modelState !== "loading") setUnloaded();
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
    if (modelState !== "ready" || busy) return;

    composer.value = "";
    const questionMessage = conversation.append("user", question);
    const answerMessage = conversation.append("assistant", "");
    const timer = createResponseTimer(answerMessage.setSteps);

    busy = true;
    followingOutput = true;
    loader.ready();
    composer.generating();
    syncControls();
    conversation.scrollToBottom();

    try {
      timer.set("Preparing context");
      await compactor.cancel();
      const draft = {
        ...chat,
        messages: [
          ...chat.messages,
          { role: /** @type {const} */ ("user"), content: question },
        ],
      };
      const {
        output,
        artifacts = [],
        metricPaths,
        apiContext,
        sourceContext,
        knowledgeContext,
        chat: preparedChat,
      } = await assistant.answer({
        question,
        history: draft.messages,
        model,
        async prepare() {
          timer.set("Preparing context");
          const prepared = await prepareContext(draft, model);
          timer.set("Routing request");
          return prepared;
        },
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
      const response = output ?? "";
      conversation.setContent(answerMessage.content, "assistant", response);
      answerMessage.setArtifacts(artifacts);
      answerMessage.setElapsed(elapsedMs);
      if (followingOutput) conversation.scrollToBottom();
      const answeredChat = preparedChat ?? draft;
      workspace = askStorage.save({
        ...answeredChat,
        messages: [
          ...answeredChat.messages,
          {
            role: /** @type {const} */ ("assistant"),
            content: response,
            elapsedMs,
            steps,
            metricPaths,
            ...(apiContext ? { apiContext } : {}),
            ...(sourceContext?.length ? { sourceContext } : {}),
            ...(knowledgeContext ? { knowledgeContext } : {}),
            ...(artifacts.length ? { artifacts } : {}),
          },
        ],
      });
      chat = workspace.activeChat;
      renderChatList();
      setReady();
      composer.focus();
      compactor.schedule(chat);
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
    compactor.stop();
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
