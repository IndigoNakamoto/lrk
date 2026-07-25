/**
 * @param {{ suggestedUnit?: string }[]} metrics
 * @param {string | undefined} existingUnit
 * @param {string} operation
 */
export function resolveChartUnit(metrics, existingUnit, operation) {
  const units = metrics.flatMap((metric) =>
    metric.suggestedUnit ? [metric.suggestedUnit] : []
  );
  if (existingUnit && operation !== "replace" && operation !== "remove") {
    units.unshift(existingUnit);
  }

  const distinct = [...new Set(units)];
  return {
    unit: operation === "remove" ? existingUnit : distinct[0] ?? existingUnit,
    conflicts: distinct.length > 1 ? distinct : [],
  };
}
