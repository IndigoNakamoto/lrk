const PREFIX = /** @type {const} */ ({
  api: "a",
  fact: "f",
  guide: "g",
  metric: "m",
  source: "s",
});

export class AskRefs {
  /** @type {Map<string, number>} */
  #counts = new Map();

  /** @type {Map<string, { kind: keyof typeof PREFIX, value: any }>} */
  #items = new Map();

  /** @type {Map<string, string>} */
  #keys = new Map();

  /**
   * @param {keyof typeof PREFIX} kind
   * @param {any} value
   * @param {string} stableKey
   */
  issue(kind, value, stableKey) {
    const key = `${kind}:${stableKey}`;
    const existing = this.#keys.get(key);
    if (existing) return existing;

    const count = (this.#counts.get(kind) ?? 0) + 1;
    const ref = `${PREFIX[kind]}${count}`;
    this.#counts.set(kind, count);
    this.#items.set(ref, { kind, value });
    this.#keys.set(key, ref);
    return ref;
  }

  /** @param {string} ref @param {keyof typeof PREFIX} [expectedKind] */
  get(ref, expectedKind) {
    const item = this.#items.get(ref);
    if (!item) throw new Error(`Unknown reference ${ref}`);
    if (expectedKind && item.kind !== expectedKind) {
      throw new Error(`${ref} is not a ${expectedKind} reference`);
    }
    return item.value;
  }

  /** @param {string} ref */
  kind(ref) {
    const item = this.#items.get(ref);
    if (!item) throw new Error(`Unknown reference ${ref}`);
    return item.kind;
  }
}
