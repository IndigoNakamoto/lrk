const WORKER_URL = import.meta.resolve("./worker.js");

/**
 * @typedef {{ name: string, arguments: Record<string, unknown> }} ToolCall
 * @typedef {{ role: "system" | "user" | "assistant" | "tool", content: string, tool_calls?: ToolCall[] }} ChatMessage
 * @typedef {{ progress: number, loaded: number, total: number }} LoadProgress
 * @typedef {{ text: string, tokensPerSecond?: number }} TokenUpdate
 * @typedef {{ text: string, toolCalls: ToolCall[], finishReason: string, tokensPerSecond?: number }} GenerationResult
 * @typedef {"auto" | "none" | { name: string }} ToolChoice
 */

export class AskModel {
  /** @type {Worker | undefined} */
  #worker;

  /** @type {((progress: LoadProgress) => void) | undefined} */
  #onProgress;

  /** @type {((status: string) => void) | undefined} */
  #onStatus;

  /** @type {((update: TokenUpdate) => void) | undefined} */
  #onToken;

  /** @type {((value: any) => void) | undefined} */
  #resolve;

  /** @type {((reason: Error) => void) | undefined} */
  #reject;

  /**
   * @param {(progress: LoadProgress) => void} onProgress
   * @param {(status: string) => void} onStatus
   */
  load(onProgress, onStatus) {
    this.#ensureWorker();
    this.#onProgress = onProgress;
    this.#onStatus = onStatus;

    return new Promise((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
      this.#worker?.postMessage({ type: "load" });
    });
  }

  isCached() {
    this.#ensureWorker();

    return /** @type {Promise<boolean>} */ (
      new Promise((resolve, reject) => {
        this.#resolve = resolve;
        this.#reject = reject;
        this.#worker?.postMessage({ type: "cache-status" });
      })
    );
  }

  /**
   * @param {ChatMessage[]} messages
   * @param {(update: TokenUpdate) => void} onToken
   * @param {readonly unknown[]} [tools]
   * @param {ToolChoice} [toolChoice]
   */
  generate(messages, onToken, tools = [], toolChoice = "auto") {
    return /** @type {Promise<GenerationResult>} */ (
      this.#request("generate", messages, onToken, tools, toolChoice)
    );
  }

  /** @param {ChatMessage[]} messages */
  async compact(messages) {
    const result = /** @type {GenerationResult} */ (
      await this.#request("compact", messages)
    );
    return result.text;
  }

  /** @param {ChatMessage[]} messages @param {readonly unknown[]} [tools] */
  countTokens(messages, tools = []) {
    return /** @type {Promise<number>} */ (
      this.#request("count", messages, undefined, tools)
    );
  }

  stop() {
    this.#worker?.postMessage({ type: "interrupt" });
  }

  reset() {
    this.#worker?.postMessage({ type: "reset" });
  }

  #ensureWorker() {
    if (this.#worker) return;

    this.#worker = new Worker(WORKER_URL, { type: "module" });
    this.#worker.addEventListener("message", this.#handleMessage);
    this.#worker.addEventListener("error", this.#handleWorkerError);
  }

  /**
   * @param {"generate" | "compact" | "count"} type
   * @param {ChatMessage[]} messages
   * @param {((update: TokenUpdate) => void) | undefined} [onToken]
   * @param {readonly unknown[]} [tools]
   * @param {ToolChoice} [toolChoice]
   */
  #request(type, messages, onToken, tools = [], toolChoice = "auto") {
    if (!this.#worker) throw new Error("Model is not loaded");

    this.#onToken = onToken;
    return new Promise((resolve, reject) => {
      this.#resolve = resolve;
      this.#reject = reject;
      this.#worker?.postMessage({
        type,
        data: { messages, tools, toolChoice },
      });
    });
  }

  terminate() {
    const reject = this.#reject;
    this.#worker?.terminate();
    this.#worker = undefined;
    this.#onProgress = undefined;
    this.#onStatus = undefined;
    this.#onToken = undefined;
    this.#resolve = undefined;
    this.#reject = undefined;
    reject?.(new Error("Model stopped"));
  }

  /** @param {MessageEvent} event */
  #handleMessage = (event) => {
    const message = event.data;

    switch (message.status) {
      case "loading":
        this.#onStatus?.(message.data);
        break;
      case "progress_total":
        this.#onProgress?.({
          progress: message.progress,
          loaded: message.loaded,
          total: message.total,
        });
        break;
      case "ready":
        this.#settle(message.status);
        break;
      case "update":
        this.#onToken?.({
          text: message.output,
          tokensPerSecond: message.tokensPerSecond,
        });
        break;
      case "complete":
        this.#settle(message.result);
        break;
      case "counted":
        this.#settle(message.count);
        break;
      case "cache-status":
        this.#settle(message.cached);
        break;
      case "error":
        this.#fail(new Error(message.data));
        break;
    }
  };

  /** @param {ErrorEvent} event */
  #handleWorkerError = (event) => {
    this.#fail(new Error(event.message || "The model worker failed"));
  };

  /** @param {any} value */
  #settle(value) {
    this.#resolve?.(value);
    this.#onToken = undefined;
    this.#resolve = undefined;
    this.#reject = undefined;
  }

  /** @param {Error} error */
  #fail(error) {
    this.#reject?.(error);
    this.#onToken = undefined;
    this.#resolve = undefined;
    this.#reject = undefined;
  }
}
