import { searchTool } from "./schemas.js";
import { AskToolSession } from "./session.js";
import { AskSource } from "./source/index.js";
import { terminateMetricIndex } from "./metrics/index.js";

const MAX_TOOL_ROUNDS = 8;

/**
 * @typedef {Object} ToolOutcome
 * @property {boolean} done
 * @property {boolean} [general]
 * @property {string} [output]
 * @property {import("../storage.js").StoredArtifact[]} [artifacts]
 */

/** @param {import("../model.js").ChatMessage[]} messages */
function latestQuestion(messages) {
  return messages.findLast((message) => message.role === "user")?.content ?? "";
}

export function createAskTools() {
  const source = new AskSource();
  /** @type {Map<string, AskToolSession>} */
  const sessions = new Map();
  /** @type {AbortController | undefined} */
  let controller;

  /** @param {string} chatId */
  function sessionFor(chatId) {
    let session = sessions.get(chatId);
    if (!session) {
      session = new AskToolSession(source);
      sessions.set(chatId, session);
    }
    return session;
  }

  return {
    toolsFor() {
      return [searchTool()];
    },

    /**
     * @param {Object} options
     * @param {string} options.chatId
     * @param {import("../storage.js").StoredMessage[]} options.history
     * @param {import("../model.js").AskModel} options.model
     * @param {import("../model.js").ChatMessage[]} options.messages
     * @param {(update: import("../model.js").TokenUpdate) => void} options.onToken
     * @param {(status: string) => void} options.onStatus
     */
    async answer({ chatId, history, model, messages, onStatus }) {
      controller = new AbortController();
      const { signal } = controller;
      const question = latestQuestion(messages);
      const session = sessionFor(chatId);
      session.begin(question, history);

      try {
        const direct = await session.tryDirect(onStatus);
        if (direct) return direct;

        for (let round = 0; round < MAX_TOOL_ROUNDS; round += 1) {
          signal.throwIfAborted();
          onStatus("Thinking…");
          const newestUser = messages.findLast((message) => message.role === "user");
          const stagedMessages = [
            { role: /** @type {const} */ ("system"), content: session.instruction() },
            ...(newestUser ? [newestUser] : []),
            ...(session.observation
              ? [{
                  role: /** @type {const} */ ("user"),
                  content: `Available source-derived result:\n${JSON.stringify(session.observation)}`,
                }]
              : []),
          ];
          const result = await model.generate(
            stagedMessages,
            () => {},
            [await session.tool()],
            { name: "next_action" },
          );
          const call = result.toolCalls[0];
          if (!call || call.name !== "next_action") {
            throw new Error("The AI did not choose a valid action");
          }

          signal.throwIfAborted();
          const outcome = /** @type {ToolOutcome} */ (
            await session.execute(call.arguments, onStatus)
          );
          if (!outcome.done) continue;
          return {
            output: outcome.output ?? "",
            artifacts: outcome.artifacts ?? [],
          };
        }
        throw new Error("The AI used too many tool steps. Try a more specific question.");
      } finally {
        controller = undefined;
        onStatus("");
      }
    },

    stop() {
      controller?.abort();
      source.terminate();
    },

    terminate() {
      controller?.abort();
      controller = undefined;
      sessions.clear();
      source.terminate();
      terminateMetricIndex();
    },
  };
}
