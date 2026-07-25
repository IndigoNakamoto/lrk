const WORKER_URL = import.meta.resolve("./worker.js");

export class AskSource {
  /** @type {Worker | undefined} */
  #worker;

  /** @type {Map<string, { resolve: (value: any) => void, reject: (error: Error) => void, onProgress?: (progress: { loaded: number, total: number }) => void }>} */
  #pending = new Map();

  prewarm() {
    return this.#request("prewarm", {});
  }

  /**
   * @param {string} query
   * @param {string | undefined} path
   * @param {(progress: { loaded: number, total: number }) => void} onProgress
   */
  search(query, path, onProgress) {
    return this.#request("search", { query, path }, onProgress);
  }

  /**
   * @param {string} question
   * @param {(progress: { loaded: number, total: number }) => void} onProgress
   */
  explain(question, onProgress) {
    return this.#request("explain", { question }, onProgress);
  }

  /**
   * @param {string} path
   * @param {number} startLine
   * @param {number} endLine
   */
  read(path, startLine, endLine) {
    return this.#request("read", { path, startLine, endLine });
  }

  /**
   * @param {"prewarm" | "search" | "read" | "explain"} type
   * @param {Record<string, unknown>} data
   * @param {((progress: { loaded: number, total: number }) => void) | undefined} [onProgress]
   */
  #request(type, data, onProgress) {
    this.#ensureWorker();
    const id = crypto.randomUUID();

    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject, onProgress });
      this.#worker?.postMessage({ id, type, data });
    });
  }

  #ensureWorker() {
    if (this.#worker) return;

    this.#worker = new Worker(WORKER_URL, { type: "module" });
    this.#worker.addEventListener("message", this.#handleMessage);
    this.#worker.addEventListener("error", this.#handleWorkerError);
  }

  terminate() {
    const error = new Error("Source search stopped");
    for (const request of this.#pending.values()) request.reject(error);
    this.#pending.clear();
    this.#worker?.terminate();
    this.#worker = undefined;
  }

  /** @param {MessageEvent} event */
  #handleMessage = (event) => {
    const message = event.data;
    const request = this.#pending.get(message.id);
    if (!request) return;

    if (message.status === "progress") {
      request.onProgress?.({ loaded: message.loaded, total: message.total });
      return;
    }

    this.#pending.delete(message.id);
    if (message.status === "complete") request.resolve(message.data);
    else request.reject(new Error(message.data));
  };

  /** @param {ErrorEvent} event */
  #handleWorkerError = (event) => {
    const error = new Error(event.message || "The source worker failed");
    for (const request of this.#pending.values()) request.reject(error);
    this.#pending.clear();
    this.#worker?.terminate();
    this.#worker = undefined;
  };
}
