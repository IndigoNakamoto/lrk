const MAX_INPUT_TOKENS = 2_200;
const KEEP_RECENT_MESSAGES = 5;

const BASE_INSTRUCTIONS =
  `You are the Bitview assistant for the BRK Bitcoin research project.
Answer clearly and concisely.
If essential context is missing, ask one brief clarification instead of guessing.
BRK is the software repository, not a cryptocurrency or token.
For BRK questions, current repository evidence supplied by tools is authoritative. Use only that evidence for source-defined behavior, APIs, terminology, and metrics. Mention relevant repository paths.
In BRK source evidence, sats means Bitcoin satoshis, not products, sales, or shares. Do not reinterpret formulas or add financial meaning that the evidence does not establish.
Use metric search before building charts. Build an inline chart when it materially helps answer a quantitative Bitcoin question.`;

const MEMORY_INSTRUCTIONS = `You maintain compact memory for a conversation.
Merge the existing memory with the newly provided dialogue.
Preserve concrete facts, names, numbers, user preferences, decisions, constraints, and unresolved questions.
Do not invent information. Remove repetition and obsolete conversational filler.
Return only the updated memory, using short factual bullet points.`;

/** @typedef {import("./storage.js").StoredChat} StoredChat */
/** @typedef {import("./model.js").AskModel} AskModel */

/** @param {StoredChat} chat @param {number} [start] */
function messagesFor(chat, start = chat.compactedCount) {
  const instructions = chat.memory
    ? `${BASE_INSTRUCTIONS}\n\nEarlier conversation memory:\n${chat.memory}`
    : BASE_INSTRUCTIONS;

  return [
    { role: /** @type {const} */ ("system"), content: instructions },
    ...chat.messages.slice(start).map(({ role, content }) => ({
      role,
      content,
    })),
  ];
}

/**
 * @param {string} memory
 * @param {StoredChat["messages"]} messages
 */
function compactionPrompt(memory, messages) {
  const dialogue = messages
    .map((message) => `${message.role.toUpperCase()}: ${message.content}`)
    .join("\n\n");

  return [
    { role: /** @type {const} */ ("system"), content: MEMORY_INSTRUCTIONS },
    {
      role: /** @type {const} */ ("user"),
      content: `Existing memory:\n${memory || "(none)"}\n\nDialogue to incorporate:\n${dialogue}`,
    },
  ];
}

/**
 * @param {StoredChat} chat
 * @param {AskModel} model
 * @param {readonly unknown[]} tools
 */
export async function prepareContext(chat, model, tools) {
  let start = chat.compactedCount;
  let messages = messagesFor(chat, start);
  let tokenCount = await model.countTokens(messages, tools);
  if (tokenCount <= MAX_INPUT_TOKENS) return { chat, messages };

  start = Math.max(start, chat.messages.length - KEEP_RECENT_MESSAGES);
  while (tokenCount > MAX_INPUT_TOKENS && start < chat.messages.length - 1) {
    start += 1;
    while (
      start < chat.messages.length - 1 &&
      chat.messages[start].role !== "user"
    ) start += 1;
    messages = messagesFor(chat, start);
    tokenCount = await model.countTokens(messages, tools);
  }
  return { chat, messages };
}

/**
 * @param {StoredChat} chat
 * @param {AskModel} model
 * @param {readonly unknown[]} tools
 * @param {AbortSignal} signal
 */
export async function compactContext(chat, model, tools, signal) {
  const tokenCount = await model.countTokens(messagesFor(chat), tools);
  signal.throwIfAborted();
  const compactThrough = chat.messages.length - KEEP_RECENT_MESSAGES;
  if (
    tokenCount <= MAX_INPUT_TOKENS ||
    compactThrough <= chat.compactedCount
  ) return undefined;

  const memory = await model.compact(
    compactionPrompt(
      chat.memory,
      chat.messages.slice(chat.compactedCount, compactThrough),
    ),
  );
  signal.throwIfAborted();
  return {
    messageCount: chat.messages.length,
    previousCompactedCount: chat.compactedCount,
    previousMemory: chat.memory,
    memory: memory.trim(),
    compactedCount: compactThrough,
  };
}
