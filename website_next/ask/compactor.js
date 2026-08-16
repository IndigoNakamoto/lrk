import { compactContext } from "./context.js";

/** @param {() => void} callback */
function whenIdle(callback) {
  return requestIdleCallback(callback);
}

/** @param {number | undefined} scheduled */
function cancelIdle(scheduled) {
  if (scheduled !== undefined) cancelIdleCallback(scheduled);
}

/**
 * @param {Object} options
 * @param {import("./model.js").AskModel} options.model
 * @param {(id: string, update: NonNullable<Awaited<ReturnType<typeof compactContext>>>) => void} options.onCompacted
 */
export function createAskCompactor({ model, onCompacted }) {
  /** @type {import("./storage.js").StoredChat | undefined} */
  let pending;
  /** @type {number | undefined} */
  let scheduled;
  /** @type {{ controller: AbortController, promise: ReturnType<typeof compactContext> } | undefined} */
  let active;

  function queue() {
    if (!pending || scheduled || active) return;
    scheduled = whenIdle(() => {
      scheduled = undefined;
      void run();
    });
  }

  async function run() {
    const target = pending;
    if (!target) return;
    pending = undefined;
    const controller = new AbortController();
    const promise = compactContext(target, model, controller.signal);
    const task = { controller, promise };
    active = task;

    try {
      const update = await promise;
      if (update) onCompacted(target.id, update);
    } catch {
      // Background memory is optional; the next foreground request remains usable.
    } finally {
      if (active === task) active = undefined;
      queue();
    }
  }

  function stop() {
    pending = undefined;
    cancelIdle(scheduled);
    scheduled = undefined;
    if (!active) return;
    active.controller.abort();
    model.stop();
  }

  return {
    /** @param {import("./storage.js").StoredChat} chat */
    schedule(chat) {
      pending = chat;
      queue();
    },
    async cancel() {
      const running = active?.promise;
      stop();
      await running?.catch(() => {});
    },
    stop,
  };
}
