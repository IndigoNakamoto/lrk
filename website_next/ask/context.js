const MAX_INPUT_TOKENS = 5_600;
const KEEP_RECENT_MESSAGES = 5;

const BASE_INSTRUCTIONS =
  "You are a helpful assistant. Answer clearly and concisely.";

const MEMORY_INSTRUCTIONS = `You maintain compact memory for a conversation.
Merge the existing memory with the newly provided dialogue.
Preserve concrete facts, names, numbers, user preferences, decisions, constraints, and unresolved questions.
Do not invent information. Remove repetition and obsolete conversational filler.
Return only the updated memory, using short factual bullet points.`;

/** @typedef {import("./storage.js").StoredChat} StoredChat */
/** @typedef {import("./model.js").AskModel} AskModel */

/** @param {StoredChat} chat */
function messagesFor(chat) {
  const instructions = chat.memory
    ? `${BASE_INSTRUCTIONS}\n\nEarlier conversation memory:\n${chat.memory}`
    : BASE_INSTRUCTIONS;

  return [
    { role: /** @type {const} */ ("system"), content: instructions },
    ...chat.messages.slice(chat.compactedCount),
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
 * @param {() => void} onCompacting
 */
export async function prepareContext(chat, model, onCompacting) {
  const messages = messagesFor(chat);
  const tokenCount = await model.countTokens(messages);
  const compactThrough = chat.messages.length - KEEP_RECENT_MESSAGES;

  if (
    tokenCount <= MAX_INPUT_TOKENS ||
    compactThrough <= chat.compactedCount
  ) {
    return { chat, messages, compacted: false };
  }

  onCompacting();
  const memory = await model.compact(
    compactionPrompt(
      chat.memory,
      chat.messages.slice(chat.compactedCount, compactThrough),
    ),
  );
  const compactedChat = {
    ...chat,
    memory: memory.trim(),
    compactedCount: compactThrough,
  };

  return {
    chat: compactedChat,
    messages: messagesFor(compactedChat),
    compacted: true,
  };
}
