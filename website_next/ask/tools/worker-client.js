export class WorkerClient {
  /** @type {Worker | undefined} */
  #worker;

  /** @type {Map<string, { resolve: (value: any) => void, reject: (error: Error) => void, onProgress?: (message: any) => void }>} */
  #pending = new Map();

  /**
   * @param {string} url
   * @param {{ failed: string, stopped: string, data?: Record<string, unknown> }} options
   */
  constructor(url, options) {
    this.url = url;
    this.options = options;
  }

  /** @type {string} */
  url;

  /** @type {{ failed: string, stopped: string, data?: Record<string, unknown> }} */
  options;

  /**
   * @param {string} type
   * @param {Record<string, unknown>} data
   * @param {((message: any) => void) | undefined} [onProgress]
   */
  request(type, data, onProgress) {
    this.#ensureWorker();
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject, onProgress });
      this.#worker?.postMessage({
        id,
        type,
        data: { ...data, ...this.options.data },
      });
    });
  }

  terminate() {
    this.#fail(new Error(this.options.stopped));
  }

  #ensureWorker() {
    if (this.#worker) return;
    this.#worker = new Worker(this.url, { type: "module" });
    this.#worker.addEventListener("message", this.#handleMessage);
    this.#worker.addEventListener("error", this.#handleError);
  }

  /** @param {MessageEvent} event */
  #handleMessage = (event) => {
    const message = event.data;
    const request = this.#pending.get(message.id);
    if (!request) return;
    if (message.status === "progress") {
      request.onProgress?.(message);
      return;
    }
    this.#pending.delete(message.id);
    if (message.status === "complete") request.resolve(message.data);
    else request.reject(new Error(message.data));
  };

  /** @param {ErrorEvent} event */
  #handleError = (event) => {
    this.#fail(new Error(event.message || this.options.failed));
  };

  /** @param {Error} error */
  #fail(error) {
    for (const request of this.#pending.values()) request.reject(error);
    this.#pending.clear();
    this.#worker?.terminate();
    this.#worker = undefined;
  }
}
