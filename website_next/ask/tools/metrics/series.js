/** @param {string} value */
function toCamelCase(value) {
  const pascal = value
    .replaceAll("-", "_")
    .split("_")
    .map((word) => word ? `${word[0].toUpperCase()}${word.slice(1)}` : "")
    .join("");
  const result = pascal ? `${pascal[0].toLowerCase()}${pascal.slice(1)}` : "";
  return /^\d/.test(result) ? `_${result}` : result;
}

/** @param {unknown} value @returns {value is Record<string, any>} */
function isObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/**
 * Flatten the server's source-derived series tree into JavaScript client paths.
 *
 * @param {unknown} tree
 * @returns {{ path: string, name: string, indexes: string[], type: string }[]}
 */
export function metricsFromSeries(tree) {
  if (!isObject(tree)) throw new Error("Unsupported series catalog");
  /** @type {{ path: string, name: string, indexes: string[], type: string }[]} */
  const metrics = [];

  /** @param {Record<string, any>} node @param {string[]} path */
  function visit(node, path) {
    if (
      typeof node.name === "string" &&
      typeof node.kind === "string" &&
      Array.isArray(node.indexes)
    ) {
      metrics.push({
        path: path.join("."),
        name: node.name,
        indexes: node.indexes,
        type: node.kind,
      });
      return;
    }
    for (const [name, child] of Object.entries(node)) {
      if (!isObject(child)) throw new Error(`Invalid series catalog entry: ${name}`);
      visit(child, [...path, toCamelCase(name)]);
    }
  }

  visit(tree, []);
  return metrics.sort((left, right) => left.path.localeCompare(right.path));
}
