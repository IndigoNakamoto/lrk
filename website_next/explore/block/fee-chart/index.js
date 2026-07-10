import { createXyChart } from "../../../chart/xy/index.js";
import { createChartPoint, createChartPoints } from "../../../chart/points.js";
import { createLinearXScale } from "../../../chart/x.js";
import { createValueYScale } from "../../../chart/y.js";
import { formatFeeRate } from "../../../utils/fee-rate.js";
import { FEE_RATE_PERCENTILES, FEE_RATE_STOPS } from "../fee-rates.js";

const VIEWBOX_HEIGHT = 180;
const FEE_AVERAGE_COLOR = "var(--white)";

/** @param {number} value */
function scaleFeeRate(value) {
  return Math.log10(value + 1);
}

/**
 * @param {readonly number[]} values
 * @returns {ChartSample[]}
 */
function createPercentileSamples(values) {
  return values.map((y, index) => ({ x: FEE_RATE_PERCENTILES[index], y }));
}

/**
 * @param {ChartSample[]} samples
 * @param {number} averageRate
 */
function createAverageSample(samples, averageRate) {
  const scaledValues = samples.map(({ y }) => scaleFeeRate(y));
  const scaledAverage = scaleFeeRate(averageRate);

  if (scaledAverage <= scaledValues[0]) {
    return { x: samples[0].x, y: averageRate };
  }

  for (let index = 1; index < scaledValues.length; index += 1) {
    if (scaledAverage > scaledValues[index]) continue;

    const previousValue = scaledValues[index - 1];
    const nextValue = scaledValues[index];
    const previousSample = samples[index - 1];
    const nextSample = samples[index];
    const span = nextValue - previousValue;
    const ratio = span ? (scaledAverage - previousValue) / span : 0;
    const previousX = /** @type {number} */ (previousSample.x);
    const nextX = /** @type {number} */ (nextSample.x);

    return {
      x: previousX + (nextX - previousX) * ratio,
      y: averageRate,
    };
  }

  return { x: samples[samples.length - 1].x, y: averageRate };
}

/**
 * @param {ChartSample[]} percentileSamples
 * @param {number} averageRate
 * @returns {FeeEntry[]}
 */
function createEntries(percentileSamples, averageRate) {
  return [
    ...percentileSamples.map((sample, index) => ({
      label: FEE_RATE_STOPS[index].label,
      sample,
      color: FEE_RATE_STOPS[index].color,
      priority: 0,
    })),
    {
      label: "avg",
      sample: createAverageSample(percentileSamples, averageRate),
      color: FEE_AVERAGE_COLOR,
      priority: 1,
    },
  ].sort((a, b) => a.sample.y - b.sample.y || a.priority - b.priority);
}

/**
 * @param {FeeEntry[]} entries
 * @returns {XySeries[]}
 */
function createSeries(entries) {
  return [
    {
      label: "range",
      color: () => "var(--gray)",
      kind: /** @type {const} */ ("line"),
      hidden: true,
    },
    ...entries.map((entry) => ({
      label: entry.label,
      color: () => entry.color,
      kind: /** @type {const} */ ("point"),
    })),
  ];
}

/**
 * @param {ChartSample[]} percentileSamples
 * @param {FeeEntry[]} entries
 * @param {ChartFrame} frame
 * @returns {XyPlottedSeries[]}
 */
function plotSeries(percentileSamples, entries, frame) {
  const scaleX = createLinearXScale(
    frame,
    percentileSamples.map(({ x }) => /** @type {number} */ (x)),
  );
  const scalePlotY = createValueYScale(
    frame,
    entries.map(({ sample }) => sample.y),
    scaleFeeRate,
  );
  const percentilePoints = createChartPoints(
    percentileSamples,
    scaleX,
    scalePlotY,
  );

  return [
    { points: percentilePoints },
    ...entries.map((entry) => {
      return {
        points: [createChartPoint(entry.sample, 0, scaleX, scalePlotY)],
        readout: entry.sample.y,
      };
    }),
  ];
}

/**
 * @param {number[]} values
 * @param {number} averageRate
 */
export function createFeeChart(values, averageRate) {
  const percentileSamples = createPercentileSamples(values);
  const entries = createEntries(percentileSamples, averageRate);
  const chart = createXyChart({
    title: "Fees",
    unit: {
      id: "sat/vB",
      name: "satoshis per virtual byte",
      format: formatFeeRate,
    },
    ariaLabel: `Fee rate percentiles from ${formatFeeRate(
      values[0],
    )} to ${formatFeeRate(values[values.length - 1])} sat/vB`,
    fallbackHeight: VIEWBOX_HEIGHT,
    series: createSeries(entries),
    plot: (frame) => plotSeries(percentileSamples, entries, frame),
    marker: false,
  });

  chart.element.dataset.feeChart = "";

  return chart;
}

/**
 * @typedef {Object} FeeEntry
 * @property {string} label
 * @property {ChartSample} sample
 * @property {string} color
 * @property {number} priority
 */
