/**
 * @param {ChartSample} sample
 * @param {number} index
 * @param {(x: ChartX, index: number) => number} scaleX
 * @param {(y: number, index: number) => number} scaleY
 * @returns {ChartPoint}
 */
export function createChartPoint(sample, index, scaleX, scaleY) {
  return {
    ...sample,
    plotX: scaleX(sample.x, index),
    plotY: scaleY(sample.y, index),
  };
}

/**
 * @param {ChartSample[]} samples
 * @param {(x: ChartX, index: number) => number} scaleX
 * @param {(y: number, index: number) => number} scaleY
 */
export function createChartPoints(samples, scaleX, scaleY) {
  return samples.map((sample, index) =>
    createChartPoint(sample, index, scaleX, scaleY),
  );
}
