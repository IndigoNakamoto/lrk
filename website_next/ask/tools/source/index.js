import { WorkerClient } from "../worker-client.js";

const WORKER_URL = import.meta.resolve("./worker.js");

export class AskSource {
  #client = new WorkerClient(WORKER_URL, {
    failed: "The source worker failed",
    stopped: "Source search stopped",
  });

  prewarm() {
    return this.#client.request("prewarm", {});
  }

  /**
   * @param {string} query
   * @param {string | undefined} path
   * @param {(progress: { loaded: number, total: number }) => void} onProgress
   */
  search(query, path, onProgress) {
    return this.#client.request(
      "search",
      { query, path },
      ({ loaded, total }) => onProgress({ loaded, total }),
    );
  }

  /**
   * @param {string} question
   * @param {(progress: { loaded: number, total: number }) => void} onProgress
   */
  explain(question, onProgress) {
    return this.#client.request(
      "explain",
      { question },
      ({ loaded, total }) => onProgress({ loaded, total }),
    );
  }

  /**
   * @param {string} path
   * @param {number} startLine
   * @param {number} endLine
   */
  read(path, startLine, endLine) {
    return this.#client.request("read", { path, startLine, endLine });
  }

  terminate() {
    this.#client.terminate();
  }
}
