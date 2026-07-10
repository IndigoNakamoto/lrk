const CHECKSUM_SEPARATOR = "#";
const WSH_SORTEDMULTI_PREFIX = "wsh(sortedmulti(";
const WSH_SORTEDMULTI_SUFFIX = "))";
const MAX_WSH_MULTISIG_KEYS = 20;

/**
 * @typedef {Object} DescriptorKey
 * @property {string} xpub
 * @property {number[]} path
 */

/**
 * @typedef {Object} SortedMultisigDescriptor
 * @property {"v0_p2wsh_sortedmulti"} script
 * @property {number} threshold
 * @property {DescriptorKey[]} keys
 */

/** @param {string} text */
function compactText(text) {
  return text.trim().replace(/\s+/g, "");
}

/** @param {string} text */
function stripDescriptorChecksum(text) {
  const value = compactText(text);
  const checksumIndex = value.indexOf(CHECKSUM_SEPARATOR);

  return checksumIndex === -1 ? value : value.slice(0, checksumIndex);
}

/** @param {string} text */
function isSupportedDescriptor(text) {
  return (
    text.startsWith(WSH_SORTEDMULTI_PREFIX) &&
    text.endsWith(WSH_SORTEDMULTI_SUFFIX)
  );
}

/** @param {string} text */
function extractOutputDescriptors(text) {
  const value = compactText(text);
  const descriptors = /** @type {string[]} */ ([]);
  let offset = 0;

  while (offset < value.length) {
    const start = value.indexOf(WSH_SORTEDMULTI_PREFIX, offset);

    if (start === -1) break;

    let depth = 0;
    let end = -1;
    let seenOpen = false;

    for (let index = start; index < value.length; index += 1) {
      const character = value[index];

      if (character === "(") {
        depth += 1;
        seenOpen = true;
      }

      if (character === ")") {
        depth -= 1;
      }

      if (seenOpen && depth === 0) {
        end = index + 1;
        break;
      }
    }

    if (end === -1) break;

    const descriptor = stripDescriptorChecksum(value.slice(start, end));

    if (isSupportedDescriptor(descriptor)) {
      descriptors.push(descriptor);
    }

    offset = end;
  }

  return descriptors;
}

/** @param {string} text */
export function isOutputDescriptor(text) {
  return extractOutputDescriptors(text).length > 0;
}

/** @param {string} text */
function readFirstOutputDescriptor(text) {
  const descriptor = extractOutputDescriptors(text)[0];

  if (!descriptor) {
    throw new Error("Unsupported output descriptor");
  }

  return descriptor;
}

/** @param {string} text */
function splitDescriptorArguments(text) {
  const values = /** @type {string[]} */ ([]);
  let bracketDepth = 0;
  let groupDepth = 0;
  let start = 0;

  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];

    if (character === "[") bracketDepth += 1;
    if (character === "]") bracketDepth -= 1;
    if (character === "(") groupDepth += 1;
    if (character === ")") groupDepth -= 1;

    if (character === "," && bracketDepth === 0 && groupDepth === 0) {
      values.push(text.slice(start, index));
      start = index + 1;
    }
  }

  values.push(text.slice(start));

  return values;
}

/** @param {string} value */
function readThreshold(value) {
  const threshold = Number(value);

  if (!Number.isSafeInteger(threshold) || threshold < 1) {
    throw new Error("Invalid multisig threshold");
  }

  return threshold;
}

/** @param {string} value */
function readNonHardenedIndex(value) {
  if (value.endsWith("'") || value.endsWith("h")) {
    throw new Error("Descriptor xpub derivation cannot be hardened");
  }

  const index = Number(value);

  if (!Number.isSafeInteger(index) || index < 0) {
    throw new Error("Invalid descriptor derivation path");
  }

  return index;
}

/** @param {string} text */
function readDescriptorKeyPath(text) {
  if (!text.startsWith("/")) {
    throw new Error("Expected a ranged descriptor key path");
  }

  const segments = text.slice(1).split("/");

  if (segments[segments.length - 1] !== "*") {
    throw new Error("Expected a descriptor wildcard path");
  }

  return segments.slice(0, -1).map(readNonHardenedIndex);
}

/**
 * @param {string} text
 * @returns {DescriptorKey}
 */
function readDescriptorKey(text) {
  let value = text;

  if (value.startsWith("[")) {
    const end = value.indexOf("]");

    if (end === -1) {
      throw new Error("Invalid descriptor key origin");
    }

    value = value.slice(end + 1);
  }

  const pathIndex = value.indexOf("/");

  if (pathIndex === -1) {
    throw new Error("Expected descriptor key derivation");
  }

  return {
    xpub: value.slice(0, pathIndex),
    path: readDescriptorKeyPath(value.slice(pathIndex)),
  };
}

/**
 * @param {string} text
 * @returns {SortedMultisigDescriptor}
 */
export function parseOutputDescriptor(text) {
  const value = readFirstOutputDescriptor(text);
  const body = value.slice(
    WSH_SORTEDMULTI_PREFIX.length,
    -WSH_SORTEDMULTI_SUFFIX.length,
  );
  const [thresholdText, ...keyTexts] = splitDescriptorArguments(body);
  const threshold = readThreshold(thresholdText);
  const keys = keyTexts.map(readDescriptorKey);

  if (
    threshold > keys.length ||
    keys.length < 1 ||
    keys.length > MAX_WSH_MULTISIG_KEYS
  ) {
    throw new Error("Invalid multisig key count");
  }

  return {
    script: "v0_p2wsh_sortedmulti",
    threshold,
    keys,
  };
}

/** @param {string} descriptorText */
function inferDescriptorBranchId(descriptorText) {
  const descriptor = parseOutputDescriptor(descriptorText);
  const branchIds = descriptor.keys.map((key) => {
    return key.path[key.path.length - 1];
  });
  const sameBranch = branchIds.every((branchId) => {
    return branchId === branchIds[0];
  });

  if (!sameBranch) return undefined;
  if (branchIds[0] === 0) return "receive";
  if (branchIds[0] === 1) return "change";
}

/** @param {string} text */
export function getOutputDescriptorBranchIds(text) {
  const branchIds = /** @type {string[]} */ ([]);

  for (const descriptor of extractOutputDescriptors(text)) {
    const branchId = inferDescriptorBranchId(descriptor);

    if (branchId && !branchIds.includes(branchId)) {
      branchIds.push(branchId);
    }
  }

  return branchIds.length ? branchIds : ["receive"];
}

/**
 * @param {string} source
 * @param {string} [branchId]
 */
export function selectOutputDescriptor(source, branchId = "receive") {
  const descriptors = extractOutputDescriptors(source);

  if (descriptors.length === 0) {
    throw new Error("Unsupported output descriptor");
  }

  return descriptors.find((descriptor) => {
    return inferDescriptorBranchId(descriptor) === branchId;
  }) ?? descriptors[0];
}
