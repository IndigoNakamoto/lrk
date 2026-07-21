import { ASK_MODEL } from "./models.js";

const TRANSFORMERS_URL =
  "https://cdn.jsdelivr.net/npm/@huggingface/transformers@4.1.0";

/** @type {any} CDN module without local declarations. */
let generator;

/** @type {any} CDN module without local declarations. */
let stoppingCriteria;

/** @type {any} CDN module without local declarations. */
let transformers;

/** @param {unknown} error */
function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

async function load() {
  const adapter = await navigator.gpu?.requestAdapter();
  if (!adapter) throw new Error("WebGPU is unavailable or no adapter was found");

  self.postMessage({ status: "loading", data: "Loading AI runtime..." });
  await loadRuntime();
  stoppingCriteria = new transformers.InterruptableStoppingCriteria();

  self.postMessage({ status: "loading", data: `Loading ${ASK_MODEL.name}...` });
  generator = await transformers.pipeline("text-generation", ASK_MODEL.modelId, {
    device: "webgpu",
    dtype: ASK_MODEL.dtype,
    revision: ASK_MODEL.revision,
    /** @param {{ status: string, progress?: number, loaded?: number, total?: number }} info */
    progress_callback: (info) => {
      if (info.status !== "progress_total") return;

      self.postMessage({
        status: "progress_total",
        progress: Number(info.progress ?? 0),
        loaded: Number(info.loaded ?? 0),
        total: Number(info.total ?? 0),
      });
    },
  });

  self.postMessage({ status: "loading", data: "Warming up WebGPU..." });
  const inputs = generator.tokenizer("a");
  await generator.model.generate({ ...inputs, max_new_tokens: 1 });
  self.postMessage({ status: "ready" });
}

async function loadRuntime() {
  transformers ??= await import(TRANSFORMERS_URL);
  transformers.env.allowLocalModels = false;
}

async function checkCache() {
  await loadRuntime();
  const cached = await transformers.ModelRegistry.is_pipeline_cached(
    "text-generation",
    ASK_MODEL.modelId,
    {
      device: "webgpu",
      dtype: ASK_MODEL.dtype,
      revision: ASK_MODEL.revision,
    },
  );
  self.postMessage({ status: "cache-status", cached });
}

/**
 * @param {{ role: string, content: string }[]} messages
 * @param {{ maxNewTokens: number, stream: boolean }} options
 */
async function generate(messages, options) {
  if (!generator || !stoppingCriteria || !transformers) {
    throw new Error("Model is not loaded");
  }

  let startedAt;
  let tokenCount = 0;
  /** @type {number | undefined} */
  let tokensPerSecond;
  const streamer = options.stream
    ? new transformers.TextStreamer(generator.tokenizer, {
        skip_prompt: true,
        skip_special_tokens: true,
        /** @param {string} output */
        callback_function: (output) => {
          self.postMessage({ status: "update", output, tokensPerSecond });
        },
        token_callback_function: () => {
          startedAt ??= performance.now();
          tokenCount += 1;

          if (tokenCount > 1) {
            tokensPerSecond =
              (tokenCount / (performance.now() - startedAt)) * 1_000;
          }
        },
      })
    : undefined;
  const cache = new transformers.DynamicCache();

  try {
    const output = await generator(messages, {
      max_new_tokens: options.maxNewTokens,
      do_sample: false,
      streamer,
      stopping_criteria: stoppingCriteria,
      past_key_values: cache,
    });

    self.postMessage({
      status: "complete",
      output: output[0].generated_text.at(-1).content,
    });
  } finally {
    cache.dispose?.();
  }
}

function reset() {
  stoppingCriteria?.reset();
}

/** @param {{ role: string, content: string }[]} messages */
function countTokens(messages) {
  if (!generator) throw new Error("Model is not loaded");

  const tokens = generator.tokenizer.apply_chat_template(messages, {
    add_generation_prompt: true,
    tokenize: true,
    return_tensor: false,
    return_dict: false,
  });
  self.postMessage({ status: "counted", count: tokens.length });
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
        stoppingCriteria?.reset();
        await generate(data, { maxNewTokens: 384, stream: true });
        break;
      case "compact":
        stoppingCriteria?.reset();
        await generate(data, { maxNewTokens: 512, stream: false });
        break;
      case "count":
        countTokens(data);
        break;
      case "interrupt":
        stoppingCriteria?.interrupt();
        break;
      case "reset":
        reset();
        break;
    }
  } catch (error) {
    self.postMessage({ status: "error", data: errorMessage(error) });
  }
});
