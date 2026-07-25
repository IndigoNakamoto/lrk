const DEFINITION = /\b([A-Za-z][A-Za-z0-9_]*)\s*=\s*(.+)$/;
const IDENTIFIER = /^[A-Za-z][A-Za-z0-9_]*$/;

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
  return words[0].toUpperCase() + words.slice(1);
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
    metric,
    formula: `${metric} = ${rawExpression.trim()}`,
    value: factorsText(value),
    weight: factorsText(weight),
    otherWeight: otherWeight ? factorsText(otherWeight) : "",
  };
}

/** @param {{ path: string, text: string }} file */
function formulasInFile(file) {
  return file.text.split("\n").flatMap((line, index) => {
    const comment = line.replace(/^\s*(?:\/\/\/?|#)\s?/, "").trim();
    const definition = comment.match(DEFINITION);
    if (!definition) return [];

    const fact = deriveFormula(definition[1], definition[2]);
    return fact ? [{ ...fact, path: file.path, line: index + 1 }] : [];
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
  return files.flatMap(formulasInFile);
}

/**
 * @param {string} question
 * @param {ReturnType<typeof createFormulaIndex>} formulas
 */
export function explainFormula(question, formulas) {
  const fact = formulas
    .map((candidate) => ({
      candidate,
      score: questionScore(question, candidate.metric),
    }))
    .filter(({ score }) => score > 0)
    .sort((a, b) => b.score - a.score)[0]?.candidate;
  if (!fact) return undefined;

  const metric = title(fact.metric);
  const values = plural(fact.value);
  const consequence = fact.otherWeight
    ? ` For the same ${fact.otherWeight}, higher ${values} have more influence on the result.`
    : "";

  return {
    answer:
      `${metric} is a weighted average of ${values}. ` +
      `Each ${fact.value} is weighted by \`${fact.weight}\`.${consequence}`,
    fact,
  };
}
