import { FEE_RATE_STOPS } from "../../fee-rates.js";

let stops = /** @type {OklabColor[] | null} */ (null);

/** @param {number} value */
const clamp01 = (value) => Math.min(1, Math.max(0, value));

/**
 * @param {string} css
 */
function parseOklch(css) {
  const [, rawLightness, percent, rawChroma, rawHue] = /** @type {RegExpMatchArray} */ (
    css.match(/oklch\(\s*([\d.]+)(%)?\s+([\d.]+)\s+([\d.]+)/)
  );
  const lightness = Number(rawLightness);
  const hue = Number(rawHue) * Math.PI / 180;
  const chroma = Number(rawChroma);

  return {
    a: chroma * Math.cos(hue),
    b: chroma * Math.sin(hue),
    l: percent ? lightness / 100 : lightness,
  };
}

/**
 * @param {string} color
 */
function resolveColor(color) {
  const variable = color.match(/var\((--[^)]+)\)/)?.[1];

  return variable === undefined
    ? color
    : getComputedStyle(document.documentElement).getPropertyValue(variable);
}

function getStops() {
  stops ??= FEE_RATE_STOPS.map(({ color }) => parseOklch(resolveColor(color)));

  return stops;
}

/**
 * @param {string} color
 */
export function getCanvasColor(color) {
  return `rgb(${toRgb(parseOklch(resolveColor(color))).join(" ")})`;
}

/**
 * @param {number} linear
 */
function encodeRgb(linear) {
  const value = linear <= 0.0031308
    ? 12.92 * linear
    : 1.055 * linear ** (1 / 2.4) - 0.055;

  return Math.round(clamp01(value) * 255);
}

/**
 * @param {OklabColor} color
 */
function toRgb(color) {
  const l = color.l + 0.3963377774 * color.a + 0.2158037573 * color.b;
  const m = color.l - 0.1055613458 * color.a - 0.0638541728 * color.b;
  const s = color.l - 0.0894841775 * color.a - 1.2914855480 * color.b;
  const l3 = l ** 3;
  const m3 = m ** 3;
  const s3 = s ** 3;

  return [
    encodeRgb(+4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3),
    encodeRgb(-1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3),
    encodeRgb(-0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3),
  ];
}

/**
 * @param {OklabColor} start
 * @param {OklabColor} end
 * @param {number} ratio
 */
function mix(start, end, ratio) {
  return {
    a: start.a + (end.a - start.a) * ratio,
    b: start.b + (end.b - start.b) * ratio,
    l: start.l + (end.l - start.l) * ratio,
  };
}

/**
 * @param {number} feeRate
 * @param {number[]} ranges
 */
export function getCanvasFeeRateColor(feeRate, ranges) {
  const colors = getStops();

  if (feeRate <= ranges[0]) return `rgb(${toRgb(colors[0]).join(" ")})`;

  for (let index = 1; index < ranges.length; index += 1) {
    if (feeRate > ranges[index]) continue;

    const previousRate = ranges[index - 1];
    const nextRate = ranges[index];
    const span = nextRate - previousRate;
    const ratio = span ? (feeRate - previousRate) / span : 0;

    return `rgb(${toRgb(mix(colors[index - 1], colors[index], ratio)).join(" ")})`;
  }

  return `rgb(${toRgb(colors[colors.length - 1]).join(" ")})`;
}

/**
 * @typedef {Object} OklabColor
 * @property {number} a
 * @property {number} b
 * @property {number} l
 */
