import { ASK_MODEL } from "./models.js";

/** @type {any} bitgpu has no local declarations. */
let engine;

/** @type {any} bitgpu/chat has no local declarations. */
let chat;

/** @type {AbortController | undefined} */
let generationController;

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

/** @param {string} url */
async function cachedResponse(url) {
  const cache = await caches.open(ASK_MODEL.cacheName);
  const cached = await cache.match(url);
  if (cached) return cached;

  const response = await fetch(url);
  if (!response.ok) throw new Error(`Could not download model file (${response.status})`);

  void cache.put(url, response.clone()).catch(() => {});
  return response;
}

/** @param {string} url */
async function fetchJson(url) {
  return (await cachedResponse(url)).json();
}

/** @param {string} url */
async function fetchArrayBuffer(url) {
  return (await cachedResponse(url)).arrayBuffer();
}

/** @param {string} url */
async function fetchStream(url) {
  const body = (await cachedResponse(url)).body;
  if (!body) throw new Error("The model download did not provide a stream");
  return body;
}

async function load() {
  if (chat) {
    self.postMessage({ status: "ready" });
    return;
  }

  if (!navigator.gpu) throw new Error("WebGPU is unavailable in this browser");

  self.postMessage({ status: "loading", data: "Loading AI runtime..." });
  const [{ createEngine }, { createChat }] = await Promise.all([
    import(ASK_MODEL.runtimeUrl),
    import(ASK_MODEL.chatUrl),
  ]);

  self.postMessage({ status: "loading", data: `Loading ${ASK_MODEL.name}...` });
  engine = await createEngine({
    manifestUrl: ASK_MODEL.manifestUrl,
    auxUrl: ASK_MODEL.auxUrl,
    dataUrl: ASK_MODEL.dataUrl,
    kvCache: "q8",
    activation: "f16",
    maxSeqLen: 4_096,
    syncSteps: 1,
    fetchJson,
    fetchArrayBuffer,
    fetchStream,
    /** @param {{ phase: string, loaded?: number, total?: number }} progress */
    onProgress(progress) {
      const loaded = Number(progress.loaded ?? 0);
      const total = Number(progress.total ?? 0);
      if (progress.phase !== "weights" || !total) return;

      self.postMessage({
        status: "progress_total",
        progress: (loaded / total) * 100,
        loaded,
        total,
      });
    },
    /** @param {{ message?: string }} info */
    onDeviceLost(info) {
      engine = undefined;
      chat = undefined;
      self.postMessage({
        status: "error",
        data: info.message || "The GPU device was lost",
      });
    },
  });
  chat = await createChat(engine, {
    tokenizerJsonUrl: ASK_MODEL.tokenizerJsonUrl,
    tokenizerConfigUrl: ASK_MODEL.tokenizerConfigUrl,
    fetchJson,
  });

  self.postMessage({ status: "ready" });
}

async function checkCache() {
  const cache = await caches.open(ASK_MODEL.cacheName);
  self.postMessage({
    status: "cache-status",
    cached: Boolean(await cache.match(ASK_MODEL.dataUrl)),
  });
}

/**
 * @param {{ role: string, content: string, tool_calls?: { name: string, arguments: Record<string, unknown> }[] }[]} messages
 * @param {{ maxTokens: number, stream: boolean, tools: readonly any[], toolChoice?: "auto" | "none" | { name: string } }} options
 */
async function generate(messages, options) {
  if (!chat) throw new Error("Model is not loaded");

  generationController = new AbortController();
  const tools = options.tools.length ? options.tools : undefined;
  const stream = options.stream && (!tools || options.toolChoice === "none");

  try {
    const result = await chat.send(messages, {
      maxTokens: options.maxTokens,
      temperature: 0,
      repetitionPenalty: 1.05,
      signal: generationController.signal,
      tools,
      toolChoice: tools ? options.toolChoice : undefined,
      onText: stream
        ? (/** @type {string} */ output) => {
            self.postMessage({ status: "update", output });
          }
        : undefined,
    });

    self.postMessage({
      status: "complete",
      result: {
        text: result.text,
        toolCalls: result.toolCalls,
        finishReason: result.finishReason,
        tokensPerSecond: result.tokensPerSecond,
      },
    });
  } finally {
    generationController = undefined;
  }
}

/** @param {{ role: string, content: string }[]} messages @param {readonly any[]} tools */
function countTokens(messages, tools) {
  if (!chat) throw new Error("Model is not loaded");
  self.postMessage({
    status: "counted",
    count: chat.countTokens(messages, { tools: tools.length ? tools : undefined }),
  });
}

self.addEventListener("message", async (event) => {
  const { type, data } = event.data;

  try {
    switch (type) {
      case "cache-status":
        await checkCache();
        break;
      case "load":
        await load();
        break;
      case "generate":
        await generate(data.messages, {
          maxTokens: 256,
          stream: true,
          tools: data.tools,
          toolChoice: data.toolChoice,
        });
        break;
      case "compact":
        await generate(data.messages, {
          maxTokens: 256,
          stream: false,
          tools: [],
        });
        break;
      case "count":
        countTokens(data.messages, data.tools);
        break;
      case "interrupt":
        generationController?.abort();
        break;
      case "reset":
        chat?.reset();
        break;
    }
  } catch (error) {
    self.postMessage({ status: "error", data: errorMessage(error) });
  }
});
