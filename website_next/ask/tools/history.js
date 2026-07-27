/** @param {any} message */
function latestMessageChart(message) {
  return message.artifacts?.findLast?.(
    (/** @type {any} */ artifact) => artifact.type === "chart",
  );
}

/** @param {any[]} history */
export function latestChart(history) {
  for (const message of [...history].reverse()) {
    const chart = latestMessageChart(message);
    if (chart) return chart;
  }
  return undefined;
}

/** @param {any[]} history @returns {string[] | undefined} */
export function latestMetricPaths(history) {
  for (const message of [...history].reverse()) {
    if (Array.isArray(message.metricPaths)) return message.metricPaths;
    const chart = latestMessageChart(message);
    if (chart) return chart.chart.series.map((/** @type {any} */ item) => item.path);
  }
  return undefined;
}

/** @param {any[]} history */
export function recentMetricPaths(history) {
  /** @type {string[]} */
  const paths = [];
  for (const message of [...history].reverse()) {
    const chart = latestMessageChart(message);
    const remembered = Array.isArray(message.metricPaths)
      ? message.metricPaths
      : chart?.chart.series.map((/** @type {any} */ item) => item.path);
    for (const path of remembered ?? []) {
      if (!paths.includes(path)) paths.push(path);
      if (paths.length === 6) return paths;
    }
  }
  return paths;
}

/** @param {any[]} history */
export function latestApiContext(history) {
  for (const message of [...history].reverse()) {
    if (message.apiContext) return message.apiContext;
    if (Array.isArray(message.metricPaths) || latestMessageChart(message)) return undefined;
  }
  return undefined;
}

/** @param {any[]} history */
export function latestSourceContext(history) {
  for (const message of [...history].reverse()) {
    if (Array.isArray(message.sourceContext) && message.sourceContext.length) {
      return message.sourceContext;
    }
    if (
      message.apiContext ||
      Array.isArray(message.metricPaths) ||
      latestMessageChart(message)
    ) return undefined;
  }
  return undefined;
}

/** @param {any[]} history */
export function latestKnowledgeContext(history) {
  for (const message of [...history].reverse()) {
    if (message.knowledgeContext) return message.knowledgeContext;
    if (
      message.apiContext ||
      Array.isArray(message.sourceContext) && message.sourceContext.length ||
      latestMessageChart(message)
    ) return undefined;
  }
  return undefined;
}
