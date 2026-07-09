/** @param {number} value */
export function formatCoordinate(value) {
  return value.toFixed(2);
}

/**
 * @param {string} command
 * @param {number} x
 * @param {number} y
 */
function createPathCommand(command, x, y) {
  return `${command}${formatCoordinate(x)} ${formatCoordinate(y)}`;
}

/** @param {ChartPoint[]} points */
export function createLinePathData(points) {
  return points
    .map(({ plotX, plotY }, index) =>
      createPathCommand(index ? "L" : "M", plotX, plotY),
    )
    .join(" ");
}

/** @param {StackedPoint[]} points */
export function createAreaPathData(points) {
  const commands = points.map(({ plotX, plotY1 }, index) =>
    createPathCommand(index ? "L" : "M", plotX, plotY1),
  );

  for (let index = points.length - 1; index >= 0; index -= 1) {
    const { plotX, plotY0 } = points[index];

    commands.push(createPathCommand("L", plotX, plotY0));
  }

  return `${commands.join(" ")} Z`;
}
