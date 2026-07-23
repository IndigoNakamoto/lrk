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
 * @property {string[]} [metricPaths]
 */

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
     * @param {string} options.question
     * @param {import("../storage.js").StoredMessage[]} options.history
     * @param {import("../model.js").AskModel} options.model
     * @param {() => Promise<{ chat: import("../storage.js").StoredChat, messages: import("../model.js").ChatMessage[] }>} options.prepare
     * @param {(update: import("../model.js").TokenUpdate) => void} options.onToken
     * @param {(status: string) => void} options.onStatus
     */
    async answer({ chatId, question, history, model, prepare, onToken, onStatus }) {
      controller = new AbortController();
      const { signal } = controller;
      const session = sessionFor(chatId);
      await session.begin(
        question,
        history,
        () => onStatus("Indexing metrics…"),
      );

      try {
        const direct = await session.tryDirect(onStatus);
        if (direct) return { ...direct, metricPaths: session.metricPaths() };

        const prepared = await prepare();
        const { messages } = prepared;

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
          if (outcome.general) {
            onStatus("Answering…");
            const answer = await model.generate(
              messages,
              onToken,
              [],
              "none",
            );
            return {
              output: answer.text,
              artifacts: [],
              metricPaths: [],
              chat: prepared.chat,
            };
          }
          return {
            output: outcome.output ?? "",
            artifacts: outcome.artifacts ?? [],
            metricPaths: session.metricPaths(),
            chat: prepared.chat,
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
