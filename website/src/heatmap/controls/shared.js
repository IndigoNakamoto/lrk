import { createPersistedValue } from "../../../scripts/utils/persisted.js";

/**
 * @param {HeatmapOption} option
 * @param {string} key
 * @param {string} urlKey
 * @param {string} defaultValue
 */
export function createHeatmapPersistedValue(option, key, urlKey, defaultValue) {
  return createPersistedValue({
    defaultValue,
    storageKey: `${heatmapStoragePrefix(option)}-${key}`,
    urlKey,
    serialize: (value) => value,
    deserialize: (value) => value,
  });
}

/**
 * @template T
 * @param {readonly T[]} choices
 * @param {string} key
 * @param {T} fallback
 * @param {(choice: T) => string} toKey
 */
export function findChoiceByKey(choices, key, fallback, toKey) {
  return (
    choices.find((candidate) => {
      if (toKey(candidate) === key) return true;
      const aliases =
        candidate &&
        typeof candidate === "object" &&
        "aliases" in candidate &&
        /** @type {{ aliases?: readonly string[] }} */ (candidate).aliases;
      return Array.isArray(aliases) && aliases.includes(key);
    }) ?? fallback
  );
}

/** @param {HeatmapOption} option */
function heatmapStoragePrefix(option) {
  return `heatmap-${option.path.join("-")}`;
}
