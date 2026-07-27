import { normalize } from "../text.js";

const ASSIGNMENT =
  /\b(?:const|let)\s+([A-Za-z_$][\w$]*)(?:\s*:[^=;]+)?\s*=\s*([^;]{1,320});/gs;

/** @param {string} value */
function compact(value) {
  return value.replace(/\s+/g, " ").trim();
}

/** @param {string} value */
function words(value) {
  return new Set(normalize(value).split(" ").filter(Boolean));
}

/** @param {{ content: string }[]} excerpts */
function assignments(excerpts) {
  return excerpts.flatMap((excerpt) =>
    [...excerpt.content.matchAll(ASSIGNMENT)].map((match) => ({
      name: match[1],
      expression: compact(match[2]),
    }))
  );
}

/** @param {{ content: string }[]} excerpts */
function functions(excerpts) {
  return excerpts.flatMap((excerpt) =>
    [...excerpt.content.matchAll(
      /\b(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(|\bfunction\s+([A-Za-z_$][\w$]*)\s*\(/g,
    )].map((match) => match[1] ?? match[2])
  );
}

/** @param {string} question @param {{ name: string, expression: string }[]} values */
function primaryAssignment(question, values) {
  const queryWords = words(question);
  return [...values].sort((left, right) => {
    /** @param {{ name: string, expression: string }} value */
    const score = (value) =>
      Number(queryWords.has(normalize(value.name))) * 10 +
      Number(/[+\-*/]/.test(value.expression)) * 3 +
      Number(/\b(?:balance|count|price|rate|value)\b/.test(normalize(value.name)));
    return score(right) - score(left);
  })[0];
}

/**
 * Produce only answers that can be stated mechanically from the selected
 * source. Everything else is left to the model.
 *
 * @param {string} question
 * @param {{ path: string, content: string }[]} excerpts
 */
export function directSourceFact(question, excerpts) {
  const text = normalize(question);
  const [excerpt] = excerpts;
  if (!excerpt) return undefined;

  if (
    /\b(?:file|path)\b/.test(text) ||
    /^(?:where|which source)\b/.test(text)
  ) {
    return `The relevant implementation is in \`${excerpt.path}\`.`;
  }

  if (/\b(?:function|method)\b/.test(text)) {
    const name = functions(excerpts).at(-1);
    if (name) return `Start with \`${name}\` in \`${excerpt.path}\`.`;
  }

  const values = assignments(excerpts);
  if (!values.length) return undefined;
  const focus = excerpts.find((item) =>
    typeof /** @type {{ focus?: unknown }} */ (item).focus === "string"
  );
  const primary = primaryAssignment(
    `${question} ${/** @type {{ focus?: string }} */ (focus)?.focus ?? ""}`,
    values,
  );
  if (!primary) return undefined;

  const referencedTerm = ["mempool", "pending"].find((term) => text.includes(term));
  if (
    referencedTerm &&
    /\b(?:include|includes|including|account|consider|use|uses)\b/.test(text)
  ) {
    const subject = values.find((value) =>
      /[+\-*/]/.test(value.expression) &&
      !normalize(`${value.name} ${value.expression}`).includes(referencedTerm)
    ) ?? primary;
    if (normalize(subject.expression).includes(referencedTerm)) {
      return `Yes. \`${subject.name}\` is computed as \`${subject.expression}\`.`;
    }
    const separate = values.find((value) =>
      value !== subject &&
      /[+\-*/?:]/.test(value.expression) &&
      normalize(`${value.name} ${value.expression}`).includes(referencedTerm)
    );
    if (separate) {
      return `No. \`${subject.name}\` is computed as \`${subject.expression}\`. ${referencedTerm === "mempool" ? "Mempool changes are" : "That value is"} handled separately as \`${separate.name} = ${separate.expression}\`.`;
    }
    return `No. \`${subject.name}\` is computed as \`${subject.expression}\`, which does not reference ${referencedTerm}.`;
  }

  if (
    /\b(?:calculat|comput|formula|implement|source)\w*\b/.test(text) ||
    /^(?:how|what|so)\b/.test(text)
  ) {
    return `\`${primary.name}\` is computed as \`${primary.expression}\`.`;
  }
  return undefined;
}

/**
 * @param {string} question
 * @param {{ content: string }[]} excerpts
 */
export function sourceFocus(question, excerpts) {
  const values = assignments(excerpts);
  if (values.length) return primaryAssignment(question, values)?.name;
  if (/\b(?:function|method)\b/.test(normalize(question))) {
    return functions(excerpts).at(-1);
  }
  return undefined;
}

/**
 * Exact code expressions are safe to answer from the first source match.
 * Location questions still need the resolver to choose among ranked files.
 *
 * @param {string} question
 * @param {{ content: string }[]} excerpts
 */
export function hasDirectSourceComputation(question, excerpts) {
  const text = normalize(question);
  if (
    /\b(?:file|path)\b/.test(text) ||
    /^(?:where|which source)\b/.test(text)
  ) return false;
  return Boolean(directSourceFact(question, /** @type {any} */ (excerpts)));
}
