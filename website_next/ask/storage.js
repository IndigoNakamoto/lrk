const INDEX_KEY = "bitview.ask.chats.v2";
const CHAT_KEY_PREFIX = "bitview.ask.chat.v2.";
const LEGACY_KEY = "bitview.ask.v1";
const NEW_CHAT_TITLE = "New chat";
const CHART_UNITS = new Set(["addresses", "blocks", "btc", "percent", "utxos", "usd"]);
const CHART_VIEWS = new Set(["line", "area", "stacked", "bar", "dots"]);
const CHART_SCALES = new Set(["linear", "log"]);
const CHART_COLORS = new Set([
  "orange",
  "white",
  "gray",
  "sky",
  "cyan",
  "teal",
  "yellow",
  "avocado",
  "amber",
  "green",
  "emerald",
  "lime",
  "rose",
  "pink",
  "fuchsia",
  "purple",
  "violet",
  "indigo",
  "blue",
  "red",
]);

/**
 * @typedef {Object} StoredMessage
 * @property {"user" | "assistant"} role
 * @property {string} content
 * @property {number} [elapsedMs]
 * @property {StoredResponseStep[]} [steps]
 * @property {StoredArtifact[]} [artifacts]
 * @property {string[]} [metricPaths]
 *
 * @typedef {Object} StoredResponseStep
 * @property {string} label
 * @property {number} elapsedMs
 *
 * @typedef {Object} ChartSeriesSpec
 * @property {string} path
 * @property {string} label
 * @property {"orange" | "white" | "gray" | "sky" | "cyan" | "teal" | "yellow" | "avocado" | "amber" | "green" | "emerald" | "lime" | "rose" | "pink" | "fuchsia" | "purple" | "violet" | "indigo" | "blue" | "red"} color
 *
 * @typedef {Object} ChartSpec
 * @property {string} title
 * @property {"addresses" | "blocks" | "btc" | "percent" | "utxos" | "usd"} unit
 * @property {"line" | "area" | "stacked" | "bar" | "dots"} view
 * @property {"linear" | "log"} scale
 * @property {ChartSeriesSpec[]} series
 *
 * @typedef {Object} ChartArtifact
 * @property {"chart"} type
 * @property {string} id
 * @property {ChartSpec} chart
 *
 * @typedef {ChartArtifact} StoredArtifact
 *
 * @typedef {Object} ChatMeta
 * @property {string} id
 * @property {string} title
 * @property {number} createdAt
 * @property {number} updatedAt
 * @property {number} messageCount
 *
 * @typedef {Object} StoredChat
 * @property {string} id
 * @property {string} title
 * @property {number} createdAt
 * @property {number} updatedAt
 * @property {number} messageCount
 * @property {StoredMessage[]} messages
 * @property {string} memory
 * @property {number} compactedCount
 *
 * @typedef {Object} ChatIndex
 * @property {string} activeChatId
 * @property {ChatMeta[]} chats
 */

/** @param {unknown} value @returns {StoredArtifact | undefined} */
function readArtifact(value) {
  if (!value || typeof value !== "object") return undefined;
  const artifact = /** @type {Record<string, any>} */ (value);
  const chart = artifact.chart;
  if (
    artifact.type !== "chart" ||
    typeof artifact.id !== "string" ||
    !chart ||
    typeof chart !== "object" ||
    typeof chart.title !== "string" ||
    typeof chart.unit !== "string" ||
    typeof chart.view !== "string" ||
    typeof chart.scale !== "string" ||
    !CHART_UNITS.has(chart.unit) ||
    !CHART_VIEWS.has(chart.view) ||
    !CHART_SCALES.has(chart.scale) ||
    !Array.isArray(chart.series)
  ) return undefined;

  const series = chart.series.filter(
    (/** @type {any} */ item) =>
      item &&
      typeof item.path === "string" &&
      typeof item.label === "string" &&
      typeof item.color === "string" &&
      CHART_COLORS.has(item.color),
  );
  if (!series.length) return undefined;

  return /** @type {StoredArtifact} */ ({
    type: "chart",
    id: artifact.id,
    chart: {
      title: chart.title,
      unit: chart.unit,
      view: chart.view,
      scale: chart.scale,
      series,
    },
  });
}

/** @param {unknown} value @returns {StoredResponseStep | undefined} */
function readResponseStep(value) {
  if (!value || typeof value !== "object") return undefined;

  const step = /** @type {Record<string, unknown>} */ (value);
  if (
    typeof step.label !== "string" ||
    typeof step.elapsedMs !== "number" ||
    !Number.isFinite(step.elapsedMs) ||
    step.elapsedMs < 0
  ) return undefined;

  return { label: step.label, elapsedMs: step.elapsedMs };
}

/** @param {unknown} value @returns {StoredMessage | undefined} */
function readMessage(value) {
  if (!value || typeof value !== "object") return undefined;

  const message = /** @type {Record<string, unknown>} */ (value);
  if (
    (message.role !== "user" && message.role !== "assistant") ||
    typeof message.content !== "string"
  ) return undefined;

  const artifacts = Array.isArray(message.artifacts)
    ? /** @type {StoredArtifact[]} */ (
        message.artifacts.map(readArtifact).filter(Boolean)
      )
    : [];
  const elapsedMs =
    typeof message.elapsedMs === "number" &&
    Number.isFinite(message.elapsedMs) &&
    message.elapsedMs >= 0
      ? message.elapsedMs
      : undefined;
  const steps = Array.isArray(message.steps)
    ? /** @type {StoredResponseStep[]} */ (
        message.steps.map(readResponseStep).filter(Boolean)
      )
    : [];
  const rawMetricPaths = message.metricPaths;
  const hasMetricPaths = Array.isArray(rawMetricPaths);
  const metricPaths = hasMetricPaths
    ? [...new Set(/** @type {string[]} */ (rawMetricPaths.filter(
        (/** @type {unknown} */ path) => typeof path === "string" && path.trim(),
      )))].slice(0, 6)
    : [];
  return {
    role: message.role,
    content: message.content,
    ...(elapsedMs !== undefined ? { elapsedMs } : {}),
    ...(steps.length ? { steps } : {}),
    ...(artifacts.length ? { artifacts } : {}),
    ...(hasMetricPaths ? { metricPaths } : {}),
  };
}

/** @param {string} question */
function titleFromQuestion(question) {
  const title = question.replace(/\s+/g, " ").trim();
  return title.length > 44 ? `${title.slice(0, 43)}…` : title;
}

/** @returns {ChatMeta} */
function createMeta() {
  const now = Date.now();
  return {
    id: crypto.randomUUID(),
    title: NEW_CHAT_TITLE,
    createdAt: now,
    updatedAt: now,
    messageCount: 0,
  };
}

/** @param {ChatMeta} meta */
function emptyChat(meta) {
  return { ...meta, messages: [], memory: "", compactedCount: 0 };
}

/** @param {string} id */
function chatKey(id) {
  return `${CHAT_KEY_PREFIX}${id}`;
}

/** @param {ChatIndex} index */
function writeIndex(index) {
  localStorage.setItem(INDEX_KEY, JSON.stringify(index));
}

/** @param {StoredChat} chat */
function writeChat(chat) {
  localStorage.setItem(
    chatKey(chat.id),
    JSON.stringify({
      messages: chat.messages,
      memory: chat.memory,
      compactedCount: chat.compactedCount,
    }),
  );
}

/** @returns {ChatIndex | undefined} */
function readIndex() {
  const value = localStorage.getItem(INDEX_KEY);
  if (!value) return undefined;

  const index = /** @type {ChatIndex} */ (JSON.parse(value));
  let migrated = false;
  const chats = index.chats.map((meta) => {
    if (Number.isInteger(meta.messageCount)) return meta;

    migrated = true;
    return { ...meta, messageCount: readChat(meta).messages.length };
  });

  if (!migrated) return index;

  const next = { ...index, chats };
  writeIndex(next);
  return next;
}

/** @param {ChatMeta} meta */
function readChat(meta) {
  const value = localStorage.getItem(chatKey(meta.id));
  if (!value) return emptyChat(meta);

  const stored = JSON.parse(value);
  const messages = Array.isArray(stored.messages)
    ? stored.messages.map(readMessage).filter(Boolean)
    : [];
  const compactedCount = Math.min(
    Number(stored.compactedCount) || 0,
    messages.length,
  );

  return {
    ...meta,
    messageCount: messages.length,
    messages,
    memory: typeof stored.memory === "string" ? stored.memory : "",
    compactedCount,
  };
}

/** @returns {ChatIndex} */
function initialize() {
  const meta = createMeta();
  const legacy = localStorage.getItem(LEGACY_KEY);
  const messages = /** @type {StoredMessage[]} */ (
    legacy ? JSON.parse(legacy).map(readMessage).filter(Boolean) : []
  );

  if (messages.length) {
    meta.messageCount = messages.length;
    const firstQuestion = messages.find((message) => message.role === "user");
    if (firstQuestion) meta.title = titleFromQuestion(firstQuestion.content);
  }

  const index = { activeChatId: meta.id, chats: [meta] };
  writeChat({ ...emptyChat(meta), messages });
  writeIndex(index);
  localStorage.removeItem(LEGACY_KEY);
  return index;
}

function load() {
  const index = readIndex() ?? initialize();
  const activeMeta =
    index.chats.find((chat) => chat.id === index.activeChatId) ?? index.chats[0];

  return {
    chats: index.chats,
    activeChat: readChat(activeMeta),
  };
}

function create() {
  const index = readIndex() ?? initialize();
  const meta = createMeta();
  const next = { activeChatId: meta.id, chats: [meta, ...index.chats] };

  writeChat(emptyChat(meta));
  writeIndex(next);
  return { chats: next.chats, activeChat: emptyChat(meta) };
}

/** @param {string} id */
function select(id) {
  const index = readIndex() ?? initialize();
  const meta = index.chats.find((chat) => chat.id === id);
  if (!meta) throw new Error("Chat not found");

  writeIndex({ ...index, activeChatId: id });
  return { chats: index.chats, activeChat: readChat(meta) };
}

/** @param {StoredChat} chat */
function save(chat) {
  const index = readIndex() ?? initialize();
  const now = Date.now();
  const firstQuestion = chat.messages.find((message) => message.role === "user");
  const title =
    chat.title === NEW_CHAT_TITLE && firstQuestion
      ? titleFromQuestion(firstQuestion.content)
      : chat.title;
  const saved = {
    ...chat,
    title,
    updatedAt: now,
    messageCount: chat.messages.length,
  };
  const meta = {
    id: saved.id,
    title: saved.title,
    createdAt: saved.createdAt,
    updatedAt: saved.updatedAt,
    messageCount: saved.messages.length,
  };
  const chats = [meta, ...index.chats.filter((item) => item.id !== chat.id)];

  writeChat(saved);
  writeIndex({ activeChatId: saved.id, chats });
  return { chats, activeChat: saved };
}

/** @param {string} id */
function remove(id) {
  const index = readIndex() ?? initialize();
  const chats = index.chats.filter((chat) => chat.id !== id);

  if (!chats.length) {
    const next = createMeta();
    writeChat(emptyChat(next));
    writeIndex({ activeChatId: next.id, chats: [next] });
  } else {
    const activeChatId =
      index.activeChatId === id ? chats[0].id : index.activeChatId;
    writeIndex({ activeChatId, chats });
  }

  localStorage.removeItem(chatKey(id));
  return load();
}

export const askStorage = /** @type {const} */ ({
  load,
  create,
  select,
  save,
  remove,
});
