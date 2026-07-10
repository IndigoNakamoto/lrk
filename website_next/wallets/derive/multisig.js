import { concatBytes } from "./bytes.js";

const OP_CHECKMULTISIG = 0xae;
const COMPRESSED_PUBLIC_KEY_BYTES = 33;

/**
 * @param {Uint8Array} left
 * @param {Uint8Array} right
 */
function compareBytes(left, right) {
  for (let index = 0; index < Math.min(left.length, right.length); index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index];
  }

  return left.length - right.length;
}

/** @param {number} value */
function encodeScriptNumber(value) {
  if (value <= 16) return Uint8Array.of(0x50 + value);

  return Uint8Array.of(0x01, value);
}

/**
 * @param {readonly Uint8Array[]} publicKeys
 * @param {number} threshold
 */
export function encodeSortedMultisigScript(publicKeys, threshold) {
  const sortedKeys = [...publicKeys].sort(compareBytes);
  const pushes = sortedKeys.map((publicKey) => {
    if (publicKey.length !== COMPRESSED_PUBLIC_KEY_BYTES) {
      throw new Error("Expected compressed multisig public keys");
    }

    return concatBytes([Uint8Array.of(COMPRESSED_PUBLIC_KEY_BYTES), publicKey]);
  });

  return concatBytes([
    encodeScriptNumber(threshold),
    ...pushes,
    encodeScriptNumber(sortedKeys.length),
    Uint8Array.of(OP_CHECKMULTISIG),
  ]);
}
