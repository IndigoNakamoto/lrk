/**
 * @template {{ plotX: number }} T
 * @param {T[]} points
 * @param {number} plotX
 * @param {(point: T) => number} read
 */
export function interpolatePlotValue(points, plotX, read) {
  if (plotX <= points[0].plotX) return read(points[0]);

  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1];
    const next = points[index];

    if (plotX > next.plotX) continue;

    const span = next.plotX - previous.plotX;
    const ratio = span ? (plotX - previous.plotX) / span : 0;

    return read(previous) + (read(next) - read(previous)) * ratio;
  }

  return read(points[points.length - 1]);
}
