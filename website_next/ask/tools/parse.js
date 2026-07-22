const TOOL_CALL_PATTERN = /<tool_call>\s*([\s\S]*?)\s*<\/tool_call>/g;

/** @param {unknown} value */
function normalizeCall(value) {
  if (!value || typeof value !== "object") {
    throw new Error("The AI returned an invalid tool call");
  }

  const call = /** @type {Record<string, unknown>} */ (value);
  const name = call.name ??
    (call.function && typeof call.function === "object"
      ? /** @type {Record<string, unknown>} */ (call.function).name
      : undefined);
  let args = call.arguments ?? call.args ??
    (call.function && typeof call.function === "object"
      ? /** @type {Record<string, unknown>} */ (call.function).arguments
      : undefined);

  if (typeof args === "string") args = JSON.parse(args);
  if (typeof name !== "string" || !args || typeof args !== "object") {
    throw new Error("The AI returned an invalid tool call");
  }

  return { name, arguments: /** @type {Record<string, unknown>} */ (args) };
}

/** @param {string} output */
export function parseToolCalls(output) {
  const matches = [...output.matchAll(TOOL_CALL_PATTERN)];
  if (matches.length) {
    return matches.flatMap((match) => {
      const value = JSON.parse(match[1]);
      return (Array.isArray(value) ? value : [value]).map(normalizeCall);
    });
  }

  if (output.includes("<tool_call")) {
    throw new Error("The AI returned an incomplete tool call");
  }

  const candidate = output
    .trim()
    .replace(/^```(?:json)?\s*/i, "")
    .replace(/\s*```$/, "");
  if (candidate.startsWith("{") && candidate.endsWith("}")) {
    const value = JSON.parse(candidate);
    if (value?.name || value?.function?.name) return [normalizeCall(value)];
  }

  return [];
}
