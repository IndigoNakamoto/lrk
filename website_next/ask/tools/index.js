import { searchTool } from "./schemas.js";
import { AskToolSession } from "./session.js";
import { AskSource } from "./source/index.js";
import {
  createApiAnswerTool,
  directApiCalculation,
  finishApiAnswer,
} from "./api/answer.js";
import { prewarmApiIndex, terminateApiIndex } from "./api/index.js";
import { prewarmMetricIndex, terminateMetricIndex } from "./metrics/index.js";
import { renderDirectApiAnswer, renderEvidence } from "./render.js";
import { directSourceFact } from "./source/answer.js";

const MAX_TOOL_ROUNDS = 8;
const GATE_PROMPT = `You are the front door for Bitview's local Bitcoin assistant.
Respond directly in at most 60 words when ordinary Bitcoin knowledge, conversation, or writing is enough.
For follow-ups, keep the subject established by the recent conversation unless the user clearly changes it. Never silently substitute a different Bitcoin topic.
Separate protocol facts from interpretations. Never claim a Bitcoin mechanism guarantees price, value, security, identity, or fraud detection. Do not say fixed supply makes price stable or prevents all inflation. On-chain records identify scripts and addresses, not verified people.
When an essential subject or previous topic is missing, output only one clarification question of at most 15 words. Never guess it, explain possibilities, or list examples.
If and only if the request needs current or historical Bitview data, a concrete public blockchain record, server/API state, metric lookup, charts, cohorts, variants, or BRK repository evidence, return exactly:
TOOLS
BRK is software, not a cryptocurrency or token.

Examples:
User: Which holder group?
Assistant: Which metric or Bitcoin concept do you mean?
User: What can you do?
Assistant: I can explain Bitcoin concepts, search current BRK source, read Bitview metrics and blockchain records, and build charts. Everything runs locally in your browser.
User: Why does Bitcoin have a fixed supply?
Assistant: Bitcoin's consensus rules cap issuance at 21 million BTC. The block subsidy halves roughly every four years, so new issuance declines until the cap is approached.
User: Tell me something about on-chain data.
Assistant: On-chain data is the public record written to Bitcoin's blockchain, including transactions, amounts, fees, and block details. Analysts aggregate it to study network activity and holder behavior.
User: Chart capitalized price.
Assistant: TOOLS
User: What fee did transaction 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b pay?
Assistant: TOOLS`;

/** @param {import("../model.js").AskModel} model @param {import("../model.js").ChatMessage[]} messages */
async function useFrontDoor(model, messages) {
  const [, ...dialogue] = messages;
  const result = await model.generate(
    [
      {
        role: "system",
        content: GATE_PROMPT,
      },
      ...dialogue,
    ],
    () => {},
    [],
    "none",
    { maxTokens: 96 },
  );
  const text = result.text.trim();
  return text === "TOOLS"
    ? { kind: "tools" }
    : { kind: "answer", text };
}

/** @param {AskToolSession} session */
function modelStatus(session) {
  if (session.stage === "rewrite") return "Refining search…";
  if (session.stage === "resolve") {
    if (session.outcome === "read_api") return "Selecting API…";
    if (session.outcome === "explain_from_verified_facts") {
      return session.options.some((option) => option.kind === "source")
        ? "Selecting source…"
        : "Selecting evidence…";
    }
    return "Selecting metrics…";
  }
  return "Understanding request…";
}

/**
 * @typedef {Object} ToolOutcome
 * @property {boolean} done
 * @property {boolean} [general]
 * @property {string} [output]
 * @property {import("../storage.js").StoredArtifact[]} [artifacts]
 * @property {string[]} [metricPaths]
 * @property {{ key: string, arguments: Record<string, unknown> }} [apiContext]
 * @property {import("../storage.js").SourceContext[]} [sourceContext]
 * @property {import("../storage.js").KnowledgeContext} [knowledgeContext]
 * @property {{ question: string, metric?: { name: string, path: string, unit?: string }, excerpts: { revision: string, path: string, startLine: number, endLine?: number, content: string }[] }} [grounding]
 * @property {{ question: string, context: import("../storage.js").KnowledgeContext }} [knowledgeGrounding]
 * @property {{ question: string, operation: { key: string, method: string, path: string, summary: string, description: string, parameters: { name: string }[], response: { fields?: { name: string, type: string, description?: string }[] } }, arguments: Record<string, unknown>, requestPath: string, data: unknown, truncated: boolean }} [apiGrounding]
 * @property {NonNullable<ToolOutcome["apiGrounding"]>[]} [apiGroundings]
 */

/**
 * @param {import("../model.js").AskModel} model
 * @param {NonNullable<ToolOutcome["apiGrounding"]>} grounding
 * @param {(status: string) => void} onStatus
 */
async function answerFromApi(model, grounding, onStatus) {
  const apiAnswer = createApiAnswerTool(grounding);
  const calculation = directApiCalculation(
    grounding.question,
    apiAnswer.fields,
    grounding,
  );
  if (calculation) return calculation;
  const direct = renderDirectApiAnswer(grounding);
  if (direct) return direct;

  onStatus("Answering from API…");
  const instruction = apiAnswer.fields.length
    ? "Answer the user's exact question using only the verified API result and schema. Call answer_from_api once. Choose calculate whenever the requested numeric value combines fields. Use operator with ordered operands for ordinary arithmetic, or terms for a signed sum. Choose answer only when no arithmetic is required. Preserve identifiers and units. Never invent missing values."
    : "Answer the user's exact question using only the verified API result and schema. Call answer_from_api once with answer. Preserve identifiers and units. Never invent missing values.";
  const answer = await model.generate(
    [
      {
        role: "system",
        content: instruction,
      },
      {
        role: "user",
        content: JSON.stringify(grounding),
      },
    ],
    () => {},
    [apiAnswer.tool],
    { name: "answer_from_api" },
    { maxTokens: 128 },
  );
  const call = answer.toolCalls[0];
  if (!call || call.name !== "answer_from_api") {
    throw new Error("The AI did not produce a valid API answer");
  }
  return finishApiAnswer(call.arguments, apiAnswer.fields, grounding);
}

/** @param {NonNullable<ToolOutcome["apiGrounding"]>[]} groundings */
function answerFromApiComparison(groundings) {
  const rows = groundings.map((grounding) => {
    const answer = renderDirectApiAnswer(grounding);
    if (!answer) return undefined;
    const label = Object.values(grounding.arguments).join(", ") || grounding.requestPath;
    return `**${label}**\n${answer.replace(/\n\nData:.*$/s, "")}`;
  });
  if (rows.some((row) => !row)) {
    throw new Error("The API comparison was ambiguous");
  }
  const operation = groundings[0].operation;
  return `${rows.join("\n\n")}\n\nData: \`${operation.method} ${operation.path}\``;
}

/**
 * @param {import("../model.js").AskModel} model
 * @param {NonNullable<ToolOutcome["grounding"]>} grounding
 * @param {(update: import("../model.js").TokenUpdate) => void} onToken
 * @param {(status: string) => void} onStatus
 */
async function answerFromSource(model, grounding, onToken, onStatus) {
  const direct = directSourceFact(grounding.question, grounding.excerpts);
  if (direct) {
    return renderEvidence({
      facts: [direct],
      sources: grounding.excerpts,
      excerpts: [],
    });
  }
  onStatus("Answering from source…");
  const instruction = grounding.metric
    ? "Explain only the verified metric in plain language in at most 45 words. Start with its exact metric name in normal words and state its verified unit. Never describe the metric as denominated in another unit; source quantities may use other units only in its calculation. Use the source excerpt only for its computation; ignore sibling metrics. Preserve every comparison direction and arithmetic operation literally. Never change unrealized into realized. Do not add unsupported details."
    : "Answer in at most 45 words using only the supplied source excerpt. Describe operations in source order. Preserve the exact subject, object, and identifiers of each relationship; never merge separate statements. Do not add background knowledge, guesses, or uncited details.";
  const answer = await model.generate(
    [
      {
        role: "system",
        content: instruction,
      },
      {
        role: "user",
        content: JSON.stringify(grounding),
      },
    ],
    onToken,
    [],
    "none",
    { maxTokens: 64 },
  );
  const text = answer.text.trim();
  if (!text || /\?\s*$/.test(text)) {
    return renderEvidence({
      facts: [],
      sources: [],
      excerpts: grounding.excerpts,
    });
  }
  return renderEvidence({
    facts: [text],
    sources: grounding.excerpts,
    excerpts: [],
  });
}

/**
 * @param {import("../model.js").AskModel} model
 * @param {NonNullable<ToolOutcome["knowledgeGrounding"]>} grounding
 * @param {(update: import("../model.js").TokenUpdate) => void} onToken
 * @param {(status: string) => void} onStatus
 */
async function answerFromKnowledge(model, grounding, onToken, onStatus) {
  onStatus("Answering from context…");
  const answer = await model.generate(
    [
      {
        role: "system",
        content: "Answer the follow-up in at most 55 words using only the verified concept description. Keep the exact subject. An analogy may simplify the description but must preserve its meaning. Do not invent benefits, risks, mechanisms, or tradeoffs. If the description is insufficient, ask one short clarification question.",
      },
      {
        role: "user",
        content: JSON.stringify(grounding),
      },
    ],
    onToken,
    [],
    "none",
    { maxTokens: 80 },
  );
  return answer.text.trim();
}

export function createAskTools() {
  const source = new AskSource();
  const contextTools = [searchTool()];
  /** @type {AbortController | undefined} */
  let controller;

  return {
    prewarm() {
      return Promise.all([
        prewarmApiIndex(),
        prewarmMetricIndex(),
        source.prewarm(),
      ]);
    },

    toolsFor() {
      return contextTools;
    },

    /**
     * @param {Object} options
     * @param {string} options.question
     * @param {import("../storage.js").StoredMessage[]} options.history
     * @param {import("../model.js").AskModel} options.model
     * @param {() => Promise<{ chat: import("../storage.js").StoredChat, messages: import("../model.js").ChatMessage[] }>} options.prepare
     * @param {(update: import("../model.js").TokenUpdate) => void} options.onToken
     * @param {(status: string) => void} options.onStatus
     */
    async answer({ question, history, model, prepare, onToken, onStatus }) {
      controller = new AbortController();
      const { signal } = controller;
      const session = new AskToolSession(source);
      await session.begin(
        question,
        history,
        () => onStatus("Indexing tools…"),
      );

      try {
        let prepared;
        if (session.verifyDirectApiIntent) {
          prepared = await prepare();
          onStatus("Understanding request…");
          const frontDoor = await useFrontDoor(model, prepared.messages);
          if (frontDoor.kind === "answer") {
            return {
              output: frontDoor.text,
              artifacts: [],
              metricPaths: [],
              chat: prepared.chat,
            };
          }
        }

        const direct = /** @type {ToolOutcome | undefined} */ (
          await session.tryDirect(onStatus, signal)
        );
        if (direct?.apiGrounding) {
          return {
            output: await answerFromApi(model, direct.apiGrounding, onStatus),
            artifacts: [],
            metricPaths: session.metricPaths(),
            apiContext: session.apiContext(),
          };
        }
        if (direct?.apiGroundings) {
          return {
            output: answerFromApiComparison(direct.apiGroundings),
            artifacts: [],
            metricPaths: session.metricPaths(),
            apiContext: session.apiContext(),
          };
        }
        if (direct?.grounding) {
          return {
            output: await answerFromSource(
              model,
              direct.grounding,
              onToken,
              onStatus,
            ),
            artifacts: [],
            metricPaths: session.metricPaths(),
            sourceContext: direct.grounding.excerpts,
          };
        }
        if (direct?.knowledgeGrounding) {
          return {
            output: await answerFromKnowledge(
              model,
              direct.knowledgeGrounding,
              onToken,
              onStatus,
            ),
            artifacts: [],
            metricPaths: session.metricPaths(),
            knowledgeContext: direct.knowledgeGrounding.context,
          };
        }
        if (direct) return { ...direct, metricPaths: session.metricPaths() };

        prepared ??= await prepare();
        const { messages } = prepared;
        if (!session.requiresTools) {
          onStatus("Understanding request…");
          const frontDoor = await useFrontDoor(model, messages);
          if (frontDoor.kind === "answer") {
            return {
              output: frontDoor.text,
              artifacts: [],
              metricPaths: [],
              chat: prepared.chat,
            };
          }
        }

        for (let round = 0; round < MAX_TOOL_ROUNDS; round += 1) {
          signal.throwIfAborted();
          onStatus(modelStatus(session));
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
            { maxTokens: 64 },
          );
          const call = result.toolCalls[0];
          if (!call || call.name !== "next_action") {
            throw new Error("The AI did not choose a valid action");
          }

          signal.throwIfAborted();
          const outcome = /** @type {ToolOutcome} */ (
            await session.execute(call.arguments, onStatus, signal)
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
          if (outcome.grounding) {
            return {
              output: await answerFromSource(
                model,
                outcome.grounding,
                onToken,
                onStatus,
              ),
              artifacts: [],
              metricPaths: session.metricPaths(),
              sourceContext: outcome.grounding.excerpts,
              chat: prepared.chat,
            };
          }
          if (outcome.apiGrounding) {
            return {
              output: await answerFromApi(
                model,
                outcome.apiGrounding,
                onStatus,
              ),
              artifacts: [],
              metricPaths: session.metricPaths(),
              apiContext: session.apiContext(),
              chat: prepared.chat,
            };
          }
          return {
            output: outcome.output ?? "",
            artifacts: outcome.artifacts ?? [],
            metricPaths: session.metricPaths(),
            apiContext: session.apiContext(),
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
      source.terminate();
      terminateApiIndex();
      terminateMetricIndex();
    },
  };
}
