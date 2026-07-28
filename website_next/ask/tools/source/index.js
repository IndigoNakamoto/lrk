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
   * @param {"definition" | "implementation" | "availability" | undefined} focus
   * @param {(progress: { loaded: number, total: number }) => void} onProgress
   */
  search(query, path, focus, onProgress) {
    return this.#client.request(
      "search",
      { query, path, focus },
      ({ loaded, total }) => onProgress({ loaded, total }),
    );
  }

  terminate() {
    this.#client.terminate();
  }
}
