/**
 * @typedef {Object} ResponseStep
 * @property {string} label
 * @property {number} [elapsedMs]
 */

/** @param {number} elapsedMs */
export function formatDuration(elapsedMs) {
  if (elapsedMs < 100) return "<0.1s";
  if (elapsedMs < 60_000) return `${(elapsedMs / 1_000).toFixed(1)}s`;

  const minutes = Math.floor(elapsedMs / 60_000);
  const seconds = Math.floor((elapsedMs % 60_000) / 1_000);
  return `${minutes}m ${seconds}s`;
}

/** @param {string} label */
function normalizeLabel(label) {
  return label
    .replace(/\s*·.*$/, "")
    .replace(/(?:\.\.\.|…)$/, "")
    .trim();
}

/** @param {(steps: ResponseStep[]) => void} onUpdate */
export function createResponseTimer(onUpdate) {
  const startedAt = performance.now();
  /** @type {{ label: string, startedAt: number } | undefined} */
  let active;
  /** @type {{ label: string, elapsedMs: number }[]} */
  const completed = [];

  function publish() {
    onUpdate(active
      ? [...completed, { label: active.label }]
      : [...completed]);
  }

  function completeActive() {
    if (!active) return;

    completed.push({
      label: active.label,
      elapsedMs: Math.round(performance.now() - active.startedAt),
    });
    active = undefined;
  }

  return {
    /** @param {string} nextLabel */
    set(nextLabel) {
      const label = normalizeLabel(nextLabel);
      if (label === active?.label) return;

      completeActive();
      if (label) active = { label, startedAt: performance.now() };
      publish();
    },
    finish() {
      completeActive();
      publish();
      return {
        elapsedMs: Math.round(performance.now() - startedAt),
        steps: [...completed],
      };
    },
  };
}
