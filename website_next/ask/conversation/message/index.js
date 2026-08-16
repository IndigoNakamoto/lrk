import { renderMarkdown } from "../../markdown.js";
import { formatDuration } from "../../timing.js";
import { createAskChart } from "./chart/index.js";

/** @typedef {"user" | "assistant"} MessageRole */

/**
 * @param {HTMLElement} content
 * @param {MessageRole} role
 * @param {string} text
 */
export function setMessageContent(content, role, text) {
  if (role === "assistant") renderMarkdown(content, text);
  else content.textContent = text;
}

/**
 * @param {MessageRole} role
 * @param {string} text
 * @param {Object} [metadata]
 * @param {import("../../storage.js").StoredArtifact[]} [metadata.artifacts]
 * @param {number} [metadata.elapsedMs]
 * @param {import("../../timing.js").ResponseStep[]} [metadata.steps]
 */
export function createAskMessage(role, text, metadata = {}) {
  const { artifacts = [], elapsedMs, steps = [] } = metadata;
  const item = document.createElement("li");
  const label = document.createElement("strong");
  const content = document.createElement("div");
  const responseSteps = role === "assistant"
    ? document.createElement("ol")
    : undefined;
  const responseTime = role === "assistant"
    ? document.createElement("small")
    : undefined;

  /** @param {import("../../storage.js").StoredArtifact[]} nextArtifacts */
  function setArtifacts(nextArtifacts) {
    item.querySelectorAll(":scope > [data-ask-chart]").forEach((chart) => {
      chart.remove();
    });
    const charts = nextArtifacts
      .filter((artifact) => artifact.type === "chart")
      .map(createAskChart);

    if (responseSteps) responseSteps.before(...charts);
    else item.append(...charts);
  }

  /** @param {import("../../timing.js").ResponseStep[]} nextSteps */
  function setSteps(nextSteps) {
    if (!responseSteps) return;

    responseSteps.replaceChildren(...nextSteps.map((step) => {
      const item = document.createElement("li");
      const name = document.createElement("span");
      const duration = document.createElement("time");

      name.textContent = step.label;
      if (step.elapsedMs === undefined) item.dataset.active = "";
      else duration.textContent = formatDuration(step.elapsedMs);
      item.append(name, duration);
      return item;
    }));
    responseSteps.hidden = !nextSteps.length;
  }

  /** @param {number | undefined} nextElapsedMs */
  function setElapsed(nextElapsedMs) {
    if (!responseTime) return;

    const visible =
      typeof nextElapsedMs === "number" &&
      Number.isFinite(nextElapsedMs) &&
      nextElapsedMs >= 0;
    responseTime.hidden = !visible;
    responseTime.textContent = visible
      ? `${formatDuration(nextElapsedMs)} total`
      : "";
  }

  item.dataset.role = role;
  label.append(role === "user" ? "You" : "Assistant");
  setMessageContent(content, role, text);
  item.append(label, content);
  if (responseSteps) {
    responseSteps.dataset.responseSteps = "";
    item.append(responseSteps);
  }
  if (responseTime) {
    responseTime.dataset.responseTime = "";
    responseTime.title = "Response time";
    item.append(responseTime);
  }
  setArtifacts(artifacts);
  setSteps(steps);
  setElapsed(elapsedMs);

  return { item, content, setArtifacts, setSteps, setElapsed };
}
