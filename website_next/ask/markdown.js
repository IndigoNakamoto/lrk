/**
 * @typedef {Object} InlinePattern
 * @property {RegExp} pattern
 * @property {(match: RegExpExecArray) => HTMLElement} create
 */

/** @type {InlinePattern[]} */
const INLINE_PATTERNS = [
  {
    pattern: /`([^`\n]+)`/,
    create(match) {
      const code = document.createElement("code");
      code.textContent = match[1];
      return code;
    },
  },
  {
    pattern: /\[([^\]\n]+)\]\(((?:https?:\/\/|\/|#)[^\s)]+)\)/,
    create(match) {
      const anchor = document.createElement("a");
      anchor.href = match[2];
      appendInline(anchor, match[1]);

      if (anchor.origin !== window.location.origin) {
        anchor.target = "_blank";
        anchor.rel = "noreferrer";
      }

      return anchor;
    },
  },
  {
    pattern: /\*\*([^*\n]+)\*\*/,
    create(match) {
      const strong = document.createElement("strong");
      appendInline(strong, match[1]);
      return strong;
    },
  },
  {
    pattern: /__([^_\n]+)__/,
    create(match) {
      const strong = document.createElement("strong");
      appendInline(strong, match[1]);
      return strong;
    },
  },
  {
    pattern: /\*([^*\n]+)\*/,
    create(match) {
      const emphasis = document.createElement("em");
      appendInline(emphasis, match[1]);
      return emphasis;
    },
  },
  {
    pattern: /_([^_\n]+)_/,
    create(match) {
      const emphasis = document.createElement("em");
      appendInline(emphasis, match[1]);
      return emphasis;
    },
  },
];

/**
 * @param {HTMLElement} parent
 * @param {string} source
 */
function appendInline(parent, source) {
  let remaining = source;

  while (remaining) {
    /** @type {{ entry: InlinePattern, match: RegExpExecArray } | undefined} */
    let token;

    for (const entry of INLINE_PATTERNS) {
      const match = entry.pattern.exec(remaining);
      if (match && (!token || match.index < token.match.index)) {
        token = { entry, match };
      }
    }

    if (!token) {
      parent.append(remaining);
      return;
    }

    const { entry, match } = token;
    parent.append(remaining.slice(0, match.index), entry.create(match));
    remaining = remaining.slice(match.index + match[0].length);
  }
}

/** @param {string} line */
function isBlockStart(line) {
  return (
    /^```/.test(line) ||
    /^#{1,4}\s/.test(line) ||
    /^>\s?/.test(line) ||
    /^(?:[-+*]|\d+\.)\s+/.test(line)
  );
}

/**
 * @param {HTMLElement} parent
 * @param {string} text
 */
function appendTextBlock(parent, text) {
  appendInline(parent, text.trim());
}

/**
 * @param {HTMLElement} target
 * @param {string} source
 */
export function renderMarkdown(target, source) {
  const lines = source.replaceAll("\r\n", "\n").split("\n");
  const fragment = document.createDocumentFragment();
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];

    if (!line.trim()) {
      index += 1;
      continue;
    }

    const fence = /^```([^\s`]*)\s*$/.exec(line);
    if (fence) {
      const pre = document.createElement("pre");
      const code = document.createElement("code");
      const content = [];
      index += 1;

      while (index < lines.length && !/^```\s*$/.test(lines[index])) {
        content.push(lines[index]);
        index += 1;
      }

      code.textContent = content.join("\n");
      if (fence[1]) code.dataset.language = fence[1];
      pre.append(code);
      fragment.append(pre);
      index += index < lines.length ? 1 : 0;
      continue;
    }

    const heading = /^(#{1,4})\s+(.+)$/.exec(line);
    if (heading) {
      const element = document.createElement(`h${heading[1].length}`);
      appendTextBlock(element, heading[2]);
      fragment.append(element);
      index += 1;
      continue;
    }

    if (/^>\s?/.test(line)) {
      const quote = document.createElement("blockquote");
      const content = [];

      while (index < lines.length && /^>\s?/.test(lines[index])) {
        content.push(lines[index].replace(/^>\s?/, ""));
        index += 1;
      }

      appendTextBlock(quote, content.join(" "));
      fragment.append(quote);
      continue;
    }

    const listItem = /^(?:([-+*])|(\d+)\.)\s+(.+)$/.exec(line);
    if (listItem) {
      const ordered = Boolean(listItem[2]);
      const list = document.createElement(ordered ? "ol" : "ul");

      while (index < lines.length) {
        const itemMatch = /^(?:([-+*])|(\d+)\.)\s+(.+)$/.exec(lines[index]);
        if (!itemMatch || Boolean(itemMatch[2]) !== ordered) break;

        const item = document.createElement("li");
        appendTextBlock(item, itemMatch[3]);
        list.append(item);
        index += 1;
      }

      fragment.append(list);
      continue;
    }

    const paragraph = document.createElement("p");
    const content = [line];
    index += 1;

    while (
      index < lines.length &&
      lines[index].trim() &&
      !isBlockStart(lines[index])
    ) {
      content.push(lines[index]);
      index += 1;
    }

    appendTextBlock(paragraph, content.join(" "));
    fragment.append(paragraph);
  }

  target.replaceChildren(fragment);
}
