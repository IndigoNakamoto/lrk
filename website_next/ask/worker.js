import { ASK_MODEL } from "./models.js";

/** @type {any} bitgpu has no local declarations. */
let engine;

/** @type {any} bitgpu/chat has no local declarations. */
let chat;

/** @type {AbortController | undefined} */
let generationController;

const PROMPT_HEADROOM = 32;
const TRIMMED = "\n[…context trimmed to fit the local model…]\n";

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

/** @param {string} text */
function firstJsonObject(text) {
  const start = text.indexOf("{");
  if (start < 0) return undefined;

  let depth = 0;
  let quoted = false;
  let escaped = false;
  for (let index = start; index < text.length; index += 1) {
    const character = text[index];
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === "\"") quoted = false;
      continue;
    }
    if (character === "\"") quoted = true;
    else if (character === "{") depth += 1;
    else if (character === "}") {
      depth -= 1;
      if (depth === 0) return text.slice(start, index + 1);
    }
  }
  return undefined;
}

/** @param {any} schema @param {unknown} value @returns {unknown} */
function sanitizeSchemaValue(schema, value) {
  if (!schema || typeof schema !== "object") return undefined;
  if (schema.type === "string") {
    if (typeof value !== "string") return undefined;
    return !schema.enum || schema.enum.includes(value) ? value : undefined;
  }
  if (schema.type === "integer") {
    if (typeof value !== "number" || !Number.isInteger(value)) return undefined;
    if (schema.minimum !== undefined && value < schema.minimum) return undefined;
    if (schema.maximum !== undefined && value > schema.maximum) return undefined;
    return value;
  }
  if (schema.type === "array") {
    if (!Array.isArray(value)) return undefined;
    const items = /** @type {unknown[]} */ (value
      .map((item) => sanitizeSchemaValue(schema.items, item))
      .filter((item) => item !== undefined));
    return [...new Set(items.map((item) => JSON.stringify(item)))]
      .map((item) => JSON.parse(item));
  }
  if (schema.type === "object") {
    if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
    const input = /** @type {Record<string, unknown>} */ (value);
    return Object.fromEntries(
      Object.entries(schema.properties ?? {})
        .map(([key, property]) => [
          key,
          sanitizeSchemaValue(property, input[key]),
        ])
        .filter(([, item]) => item !== undefined),
    );
  }
  return value;
}

/**
 * BitGPU 0.19.1 can return a valid multi-tool call in text while leaving
 * toolCalls empty. Normalize that transport quirk against the supplied schema.
 *
 * @param {any} result
 * @param {readonly any[] | undefined} tools
 */
function normalizedToolCalls(result, tools) {
  const available = new Map(
    (tools ?? []).map((tool) => [tool.function?.name, tool.function]),
  );
  const calls = Array.isArray(result.toolCalls) ? result.toolCalls : [];
  const recovered = calls.length
    ? calls
    : (() => {
        const raw = firstJsonObject(String(result.text ?? ""));
        if (!raw) return [];
        try {
          const parsed = JSON.parse(raw);
          return typeof parsed.name === "string" &&
              parsed.arguments &&
              typeof parsed.arguments === "object"
            ? [{ name: parsed.name, arguments: parsed.arguments, raw }]
            : [];
        } catch {
          return [];
        }
      })();

  return recovered.flatMap((/** @type {any} */ call) => {
    const tool = available.get(call.name);
    if (!tool) return [];
    const arguments_ = sanitizeSchemaValue(
      tool.parameters,
      call.arguments,
    );
    if (!arguments_ || typeof arguments_ !== "object") return [];
    return [{ ...call, arguments: arguments_ }];
  });
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

  if (!/** @type {any} */ (navigator).gpu) {
    throw new Error("WebGPU is unavailable in this browser");
  }

  self.postMessage({ status: "loading", data: "Loading AI runtime..." });
  // BitGPU 0.19.1's smaller prefill segments keep the UI responsive during
  // long grounded prompts. Replace this compatibility hook when BitGPU
  // exposes the segment size as a public engine option.
  /** @type {any} */ (globalThis).__SEG = 64;
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
    maxSeqLen: ASK_MODEL.maxSeqLen,
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

/** @param {string} content @param {number} length */
function trimContent(content, length) {
  if (length >= content.length) return content;
  if (length <= TRIMMED.length) return content.slice(0, Math.max(0, length));

  const available = length - TRIMMED.length;
  const head = Math.ceil(available * 0.6);
  return `${content.slice(0, head)}${TRIMMED}${content.slice(-(available - head))}`;
}

/**
 * Fit every request against the same tokenizer and sequence limit used by
 * BitGPU. Old complete turns go first; only an individually oversized newest
 * message is shortened.
 *
 * @param {{ role: string, content: string, tool_calls?: { name: string, arguments: Record<string, unknown> }[] }[]} messages
 * @param {readonly any[] | undefined} tools
 * @param {number} maxTokens
 */
function fitMessages(messages, tools, maxTokens) {
  const limit = ASK_MODEL.maxSeqLen - maxTokens - PROMPT_HEADROOM;
  let fitted = messages.map((message) => ({ ...message }));
  const count = () => chat.countTokens(fitted, { tools });

  while (count() > limit) {
    const assistant = fitted
      .slice(1, -1)
      .findIndex((message) => message.role === "assistant");
    if (assistant < 0) break;

    let end = assistant + 2;
    while (end < fitted.length - 1 && fitted[end].role !== "user") end += 1;
    fitted.splice(1, end - 1);
  }
  if (count() <= limit) return fitted;

  while (count() > limit) {
    const index = fitted
      .map((message, index_) => ({ index: index_, length: message.content.length }))
      .filter(({ index: index_, length }) =>
        fitted[index_].role !== "system" && length > 0
      )
      .sort((left, right) => right.length - left.length)[0]?.index;
    if (index === undefined) {
      throw new Error("The assistant instructions exceed the local model context window");
    }

    const content = fitted[index].content;
    let low = 0;
    let high = content.length;
    while (low < high) {
      const length = Math.ceil((low + high) / 2);
      fitted[index].content = trimContent(content, length);
      if (count() <= limit) low = length;
      else high = length - 1;
    }
    fitted[index].content = trimContent(content, low);
  }
  return fitted;
}

/**
 * @param {{ role: string, content: string, tool_calls?: { name: string, arguments: Record<string, unknown> }[] }[]} messages
 * @param {{ maxTokens: number, stream: boolean, tools: readonly any[], toolChoice?: "auto" | "none" | { name: string } }} options
 */
async function generate(messages, options) {
  if (!chat) throw new Error("Model is not loaded");

  generationController = new AbortController();
  const tools = options.tools.length ? options.tools : undefined;
  const promptTools = tools && options.toolChoice !== "none" ? tools : undefined;
  const stream = options.stream && (!tools || options.toolChoice === "none");
  const fitted = fitMessages(messages, promptTools, options.maxTokens);

  try {
    const result = await chat.send(fitted, {
      maxTokens: options.maxTokens,
      temperature: 0,
      repetitionPenalty: 1,
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
        toolCalls: normalizedToolCalls(result, tools),
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
          maxTokens: data.maxTokens ?? 256,
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
