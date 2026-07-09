import * as leanQr from "../modules/lean-qr/2.7.1/index.mjs";

/**
 * @typedef {Object} QrDataUrlOptions
 * @property {number} [padX]
 * @property {number} [padY]
 * @property {number} [scale]
 */

/**
 * @typedef {Object} QrCode
 * @property {(options?: QrDataUrlOptions) => string} toDataURL
 */

const generateQr =
  /** @type {(value: string) => QrCode | undefined} */ (
    /** @type {unknown} */ (leanQr.generate)
  );

/**
 * @param {string} value
 * @param {QrDataUrlOptions} [options]
 */
export function createQrDataUrl(value, options) {
  return generateQr(value)?.toDataURL(options) ?? "";
}
