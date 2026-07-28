import { apiByKey } from "../api/index.js";
import { metricsByPaths } from "../metrics/index.js";

/** @param {any} message */
function chartFrom(message) {
  return message?.artifacts?.findLast?.(
    (/** @type {any} */ artifact) => artifact.type === "chart",
  );
}

/**
 * Only the latest assistant response owns conversational tool context. This
 * prevents an unrelated old chart, API call, or source result from silently
 * becoming active again.
 *
 * @param {import("../../storage.js").StoredMessage[]} history
 * @param {() => void} onProgress
 */
export async function loadSessionContext(history, onProgress) {
  const assistantMessages = [...history].reverse().filter(
    ({ role }) => role === "assistant",
  );
  const message = assistantMessages[0];
  if (!message) {
    return {
      metrics: [],
      recentMetrics: [],
      source: [],
    };
  }

  const chart = chartFrom(message);
  const activePaths = [...new Set([
    ...(message.metricPaths ?? []),
    ...(chart?.chart.series.map((/** @type {any} */ series) => series.path) ?? []),
  ])];
  const recentPaths = [...new Set(
    assistantMessages.slice(1, 4).flatMap((recent) => [
      ...(recent.metricPaths ?? []),
      ...(chartFrom(recent)?.chart.series.map(
        (/** @type {any} */ series) => series.path,
      ) ?? []),
    ]),
  )].filter((path) => !activePaths.includes(path));
  const paths = [...activePaths, ...recentPaths];
  const [metrics, operation] = await Promise.all([
    paths.length ? metricsByPaths(paths, onProgress) : [],
    message.apiContext?.key ? apiByKey(message.apiContext.key) : undefined,
  ]);

  return {
    ...(chart ? { chart } : {}),
    metrics: metrics.filter(({ path }) => activePaths.includes(path)),
    recentMetrics: metrics.filter(({ path }) => recentPaths.includes(path)),
    ...(operation
      ? {
          api: {
            operation,
            arguments: message.apiContext?.arguments ?? {},
            fields: message.apiContext?.fields ?? [],
          },
        }
      : {}),
    source: message.sourceContext ?? [],
    ...(message.knowledgeContext
      ? { knowledge: message.knowledgeContext }
      : {}),
  };
}
