import { formatDuration } from "./timing.js";

/** @param {import("./storage.js").StoredArtifact} artifact */
function artifactMarkdown(artifact) {
  const { chart } = artifact;
  return [
    `**Chart: ${chart.title}**`,
    `- Unit: ${chart.unit}`,
    `- View: ${chart.view}`,
    `- Scale: ${chart.scale}`,
    "- Series:",
    ...chart.series.map((series) => `  - ${series.label}: \`${series.path}\``),
  ].join("\n");
}

/** @param {import("./storage.js").StoredMessage} message */
function timingMarkdown(message) {
  const values = (message.steps ?? []).map(
    (step) => `${step.label} ${formatDuration(step.elapsedMs)}`,
  );
  if (message.elapsedMs !== undefined) {
    values.unshift(`${formatDuration(message.elapsedMs)} total`);
  }
  return values.length ? `_${values.join(" · ")}_` : "";
}

/** @param {import("./storage.js").StoredChat} chat */
export function conversationMarkdown(chat) {
  const sections = [];

  for (const message of chat.messages) {
    const values = [
      `**${message.role === "user" ? "You" : "Assistant"}**`,
      message.content.trim(),
      ...(message.artifacts ?? []).map(artifactMarkdown),
      timingMarkdown(message),
    ].filter(Boolean);
    sections.push(values.join("\n\n"));
  }
  return `${sections.join("\n\n")}\n`;
}
