const DEFINITION = /\b([A-Za-z][A-Za-z0-9_]*)\s*=\s*(.+)$/;
const IDENTIFIER = /^[A-Za-z][A-Za-z0-9_]*$/;
const DIFFERENCE =
  /^\s*let\s+([A-Za-z][A-Za-z0-9_]*)\s*=\s*\(\s*([A-Za-z][A-Za-z0-9_.]*)\s*-\s*([A-Za-z][A-Za-z0-9_.]*)\s*\)(?:\.as_[A-Za-z0-9_]+\(\)|\s+as\s+[A-Za-z0-9_]+)?\s*;/;
const ACCUMULATION =
  /^\s*\*[A-Za-z][A-Za-z0-9_.]*\.([A-Za-z][A-Za-z0-9_]*)\(\)\s*\+=\s*([A-Za-z][A-Za-z0-9_]*)\s*\*\s*([A-Za-z][A-Za-z0-9_.]*)\s*;/;
const CONDITION =
  /^(\s*)if\s+([A-Za-z][A-Za-z0-9_.]*)\s*(<=|>=|<|>)\s*([A-Za-z][A-Za-z0-9_.]*)\s*\{/;
const ELSE = /^(\s*)}\s*else\s*\{/;
const INVERSE_OPERATOR = new Map([
  ["<=", ">"],
  [">=", "<"],
  ["<", ">="],
  [">", "<="],
]);
const OPERATOR_TEXT = new Map([
  ["<=", "is at or below"],
  [">=", "is at or above"],
  ["<", "is below"],
  [">", "is above"],
]);

/**
 * @typedef {Object} FormulaFact
 * @property {"weighted" | "accumulator" | "ratio"} kind
 * @property {string} metric
 * @property {string} formula
 * @property {string} path
 * @property {number} line
 * @property {string} [value]
 * @property {string} [weight]
 * @property {string} [otherWeight]
 * @property {string} [summary]
 * @property {string} [numerator]
 * @property {string} [denominator]
 */

/** @param {string} value */
function cleanExpression(value) {
  return value
    .replace(/[`.;]+$/g, "")
    .replace(/[·×]/g, "*")
    .replace(/([A-Za-z][A-Za-z0-9_]*)²/g, "$1^2")
    .replace(/\s+/g, "")
    .replace(/^sum/i, "Σ");
}

/** @param {string} value @param {string} operator */
function splitTopLevel(value, operator) {
  let depth = 0;

  for (let index = 0; index < value.length; index += 1) {
    const character = value[index];
    if (character === "(") depth += 1;
    else if (character === ")") depth -= 1;
    else if (character === operator && depth === 0) {
      return [value.slice(0, index), value.slice(index + 1)];
    }
  }

  return undefined;
}

/** @param {string} value */
function unwrapSum(value) {
  return value.match(/^Σ\((.+)\)$/)?.[1];
}

/** @param {string} value */
function factorMap(value) {
  /** @type {Map<string, number>} */
  const factors = new Map();

  for (const rawFactor of value.split("*")) {
    const match = rawFactor.match(/^([A-Za-z][A-Za-z0-9_]*)(?:\^(\d+))?$/);
    if (!match) return undefined;

    const [, name, rawPower] = match;
    const power = Number(rawPower ?? 1);
    if (!IDENTIFIER.test(name) || power < 1) return undefined;
    factors.set(name, (factors.get(name) ?? 0) + power);
  }

  return factors;
}

/** @param {Map<string, number>} total @param {Map<string, number>} part */
function subtractFactors(total, part) {
  const result = new Map(total);

  for (const [name, power] of part) {
    const remaining = (result.get(name) ?? 0) - power;
    if (remaining < 0) return undefined;
    if (remaining === 0) result.delete(name);
    else result.set(name, remaining);
  }

  return result;
}

/** @param {Map<string, number>} factors */
function factorsText(factors) {
  return [...factors]
    .flatMap(([name, power]) => Array.from({ length: power }, () => name))
    .join(" × ");
}

/** @param {string} value */
function title(value) {
  const words = value.replace(/_/g, " ");
  if (!words.includes(" ") && words.length <= 4) return words.toUpperCase();
  return words[0].toUpperCase() + words.slice(1);
}

/** @param {string} value */
function readableIdentifier(value) {
  const identifier = value.split(".").at(-1) ?? value;
  return identifier
    .replace(/_(?:u|i|f)(?:8|16|32|64|128|size)$/i, "")
    .replace(/_/g, " ");
}

/** @param {string} value */
function plural(value) {
  if (value.includes(" × ")) return `${value} values`;
  return value.endsWith("s") ? value : `${value}s`;
}

/** @param {string} metric @param {string} rawExpression */
function deriveFormula(metric, rawExpression) {
  const expression = cleanExpression(rawExpression);
  const division = splitTopLevel(expression, "/");
  if (!division) return undefined;

  const numeratorExpression = unwrapSum(division[0]);
  const denominatorExpression = unwrapSum(division[1]);
  if (!numeratorExpression || !denominatorExpression) return undefined;

  const numerator = factorMap(numeratorExpression);
  const weight = factorMap(denominatorExpression);
  if (!numerator || !weight) return undefined;

  const value = subtractFactors(numerator, weight);
  if (!value?.size) return undefined;

  const otherWeight = subtractFactors(weight, value);
  return {
    kind: /** @type {const} */ ("weighted"),
    metric,
    formula: `${metric} = ${rawExpression.trim()}`,
    value: factorsText(value),
    weight: factorsText(weight),
    otherWeight: otherWeight ? factorsText(otherWeight) : "",
  };
}

/** @param {{ path: string, text: string }} file */
function formulasInFile(file) {
  const lines = file.text.split("\n");
  /** @type {FormulaFact[]} */
  const comments = lines.flatMap((line, index) => {
    const comment = line.replace(/^\s*(?:\/\/\/?|#)\s?/, "").trim();
    const definition = comment.match(DEFINITION);
    if (!definition) return [];

    const fact = deriveFormula(definition[1], definition[2]);
    return fact ? [{ ...fact, path: file.path, line: index + 1 }] : [];
  });
  return [...comments, ...accumulatorFormulas(file.path, lines)];
}

/**
 * Extract named metrics that are source aliases of the generic price ratio.
 * The ratio implementation itself verifies that it is close price / metric price.
 * @param {{ path: string, text: string }} file
 */
function ratioAliasesInFile(file) {
  const aliases =
    file.text.matchAll(
      /\blet\s+([A-Za-z][A-Za-z0-9_]*)\s*=\s*LazyPerBlock::from_lazy[\s\S]{0,300}?&([A-Za-z][A-Za-z0-9_]*)\.ratio\b/g,
    );
  return [...aliases].map((match) => {
    const [, metric, source] = match;
    const imported = file.text.match(
      new RegExp(
        `\\blet\\s+${source}(?:\\s*:[^=;]+)?\\s*=\\s*cfg\\.import\\(\\s*"([^"]+)"`,
      ),
    )?.[1];
    const denominator = readableIdentifier(imported ?? `${source}_price`);
    return {
      kind: /** @type {const} */ ("ratio"),
      metric,
      formula: `${metric} = spot_price / ${imported ?? `${source}_price`}`,
      numerator: "spot price",
      denominator,
      summary: `divides spot price by ${denominator}`,
      path: file.path,
      line: file.text.slice(0, match.index).split("\n").length,
    };
  });
}

/**
 * @param {string[]} lines
 * @param {number} index
 */
function enclosingCondition(lines, index) {
  const accumulationIndent = lines[index].match(/^\s*/)?.[0].length ?? 0;
  let elseIndent;
  for (let cursor = index - 1; cursor >= Math.max(0, index - 16); cursor -= 1) {
    const alternate = lines[cursor].match(ELSE);
    if (alternate && alternate[1].length < accumulationIndent) {
      elseIndent = alternate[1].length;
      continue;
    }

    const condition = lines[cursor].match(CONDITION);
    if (!condition) continue;
    const indent = condition[1].length;
    if (elseIndent === undefined && indent >= accumulationIndent) continue;
    if (elseIndent !== undefined && indent !== elseIndent) continue;
    const operator = elseIndent === undefined
      ? condition[3]
      : INVERSE_OPERATOR.get(condition[3]);
    if (!operator) return undefined;
    return {
      left: readableIdentifier(condition[2]),
      operator,
      right: readableIdentifier(condition[4]),
    };
  }
  return undefined;
}

/** @param {string} path @param {string[]} lines */
function accumulatorFormulas(path, lines) {
  return lines.flatMap((line, index) => {
    const accumulation = line.match(ACCUMULATION);
    if (!accumulation) return [];

    const [, metric, differenceName, amountName] = accumulation;
    let difference;
    for (let cursor = index - 1; cursor >= Math.max(0, index - 5); cursor -= 1) {
      const candidate = lines[cursor].match(DIFFERENCE);
      if (candidate?.[1] === differenceName) {
        difference = candidate;
        break;
      }
    }
    if (!difference) return [];

    const left = readableIdentifier(difference[2]);
    const right = readableIdentifier(difference[3]);
    const amount = readableIdentifier(amountName);
    const condition = enclosingCondition(lines, index);
    const conditionText = condition
      ? ` for entries where ${condition.left} ${OPERATOR_TEXT.get(condition.operator)} ${condition.right}`
      : "";
    const summary =
      `sums \`(${left} − ${right}) × ${amount}\`${conditionText}`;

    return [{
      kind: /** @type {const} */ ("accumulator"),
      metric,
      formula: `${metric} += (${left} - ${right}) * ${amount}`,
      summary,
      path,
      line: index + 1,
    }];
  });
}

/** @param {string} question @param {string} metric */
function questionScore(question, metric) {
  const normalizedQuestion = question.toLowerCase().replace(/[_-]+/g, " ");
  const normalizedMetric = metric.toLowerCase().replace(/_/g, " ");
  if (normalizedQuestion.includes(normalizedMetric)) return normalizedMetric.length + 10;

  const words = normalizedMetric.split(" ");
  return words.every((word) => normalizedQuestion.includes(word)) ? words.length : 0;
}

/** @param {{ path: string, text: string }[]} files */
export function createFormulaIndex(files) {
  const verifiesPriceRatio = files.some(({ text }) =>
    text.includes("Compute ratio from close price and this metric's price") &&
    /f64::from\(close\)\s*\/\s*f64::from\(price\)/.test(text)
  );
  return /** @type {FormulaFact[]} */ ([
    ...files.flatMap(formulasInFile),
    ...(verifiesPriceRatio ? files.flatMap(ratioAliasesInFile) : []),
  ]);
}

/**
 * @param {string} question
 * @param {ReturnType<typeof createFormulaIndex>} formulas
 */
export function explainFormula(question, formulas) {
  const fact = formulas
    .map((candidate, index) => ({
      candidate,
      index,
      score: questionScore(question, candidate.metric),
    }))
    .filter(({ score }) => score > 0)
    .sort((a, b) => b.score - a.score || b.index - a.index)[0]?.candidate;
  if (!fact) return undefined;

  const metric = title(fact.metric);
  if (fact.kind === "accumulator") {
    return {
      answer: `${metric} ${fact.summary ?? "is accumulated from source values"}.`,
      fact,
    };
  }
  if (fact.kind === "ratio") {
    return {
      answer:
        `${metric} is ${fact.numerator} divided by ${fact.denominator}. ` +
        `It compares Bitcoin's current market price with that metric's price basis.`,
      fact,
    };
  }

  const value = fact.value ?? "";
  const weight = fact.weight ?? "";
  const values = plural(value);
  const consequence = fact.otherWeight
    ? ` For the same ${fact.otherWeight}, higher ${values} have more influence on the result.`
    : "";

  return {
    answer:
      `${metric} is a weighted average of ${values}. ` +
      `Each ${value} is weighted by \`${weight}\`.${consequence}`,
    fact,
  };
}
