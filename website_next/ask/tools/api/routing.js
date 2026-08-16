import { normalize } from "../text.js";

/**
 * Reuse only arguments whose names are valid for the selected generated
 * OpenAPI operation. No meaning is inferred from the user's wording here.
 *
 * @param {import("./index.js").ApiOperation} operation
 * @param {{ operation: import("./index.js").ApiOperation, arguments: Record<string, unknown> } | undefined} previous
 */
export function reusableArguments(operation, previous) {
  if (!previous) return undefined;
  const required = operation.parameters.filter((parameter) => parameter.required);
  if (!required.length && previous.operation.key !== operation.key) return undefined;
  if (!required.every((parameter) => Object.hasOwn(previous.arguments, parameter.name))) {
    return undefined;
  }
  return Object.fromEntries(
    operation.parameters
      .filter((parameter) => Object.hasOwn(previous.arguments, parameter.name))
      .map((parameter) => [parameter.name, previous.arguments[parameter.name]]),
  );
}

/**
 * Copy unambiguous primitive values directly from the newest request according
 * to the generated OpenAPI parameter schema. This is syntax extraction only;
 * operation meaning still comes from generated search metadata and the model.
 *
 * @param {import("./index.js").ApiOperation} operation
 * @param {string} request
 */
export function explicitArguments(operation, request) {
  const requiredNumeric = operation.parameters.filter((parameter) =>
    parameter.required &&
    (parameter.primitive === "integer" || parameter.primitive === "number")
  );
  /** @type {Record<string, unknown>} */
  const arguments_ = {};
  if (requiredNumeric.length === 1) {
    const matches = request.match(
      /(?<![A-Za-z0-9])[-+]?\d[\d,]*(?:\.\d+)?(?![A-Za-z0-9])/g,
    ) ?? [];
    const values = [...new Set(matches.map((value) => value.replaceAll(",", "")))];
    if (values.length === 1) {
      const value = Number(values[0]);
      const parameter = requiredNumeric[0];
      if (
        Number.isFinite(value) &&
        (parameter.primitive !== "integer" || Number.isInteger(value))
      ) {
        arguments_[parameter.name] = values[0];
      }
    }
  }
  const missing = operation.parameters.filter((parameter) =>
    parameter.required && !Object.hasOwn(arguments_, parameter.name)
  );
  if (missing.length === 1 && missing[0].primitive === "string") {
    const identifiers = [...new Set(
      (request.match(/[A-Za-z0-9][A-Za-z0-9:_-]{11,}/g) ?? [])
        .filter((value) => /[A-Za-z]/.test(value) && /\d/.test(value)),
    )];
    if (identifiers.length === 1) {
      arguments_[missing[0].name] = identifiers[0];
    }
  }
  for (const parameter of operation.parameters) {
    if (Object.hasOwn(arguments_, parameter.name) || !parameter.enum?.length) {
      continue;
    }
    const query = ` ${normalize(request)} `;
    const matches = parameter.enum.filter((value) => {
      const candidate = normalize(value);
      return candidate && query.includes(` ${candidate} `);
    });
    if (matches.length === 1) arguments_[parameter.name] = matches[0];
  }
  return arguments_;
}

/**
 * @param {import("./index.js").ApiOperation} operation
 * @param {Record<string, unknown>} arguments_
 */
export function hasRequiredArguments(operation, arguments_) {
  return operation.parameters
    .filter((parameter) => parameter.required)
    .every((parameter) => Object.hasOwn(arguments_, parameter.name));
}

/**
 * Keep only generated values that were copied from the newest request or from
 * verified context. Source-derived explicit values take precedence.
 *
 * @param {import("./index.js").ApiOperation} operation
 * @param {Record<string, unknown>} candidate
 * @param {string} request
 * @param {{ operation: import("./index.js").ApiOperation, arguments: Record<string, unknown> } | undefined} previous
 */
export function validatedArguments(operation, candidate, request, previous) {
  const reused = reusableArguments(operation, previous) ?? {};
  const explicit = explicitArguments(operation, request);
  const allowed = new Set(operation.parameters.map(({ name }) => name));
  const normalizedRequest = ` ${normalize(request)} `;
  const copied = Object.fromEntries(
    Object.entries(candidate).filter(([name, value]) => {
      if (!allowed.has(name)) return false;
      if (Object.hasOwn(reused, name) || Object.hasOwn(explicit, name)) return true;
      const raw = String(value);
      const normalized = normalize(raw);
      return request.includes(raw) ||
        Boolean(normalized && normalizedRequest.includes(` ${normalized} `));
    }),
  );
  return {
    ...copied,
    ...reused,
    ...explicit,
  };
}
