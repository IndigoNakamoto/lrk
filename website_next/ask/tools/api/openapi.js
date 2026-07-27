const MAX_DESCRIPTION = 600;
const MAX_FIELD_DESCRIPTION = 240;
const MAX_PARAMETER_DESCRIPTION = 300;
const MAX_FIELDS = 32;

/** @param {unknown} value @returns {value is Record<string, any>} */
function isObject(value) {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/** @param {Record<string, any>} spec @param {string} reference */
function resolveRef(spec, reference) {
  if (!reference.startsWith("#/")) return undefined;
  return reference
    .slice(2)
    .split("/")
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, key) => isObject(value) ? value[key] : undefined, spec);
}

/** @param {Record<string, any>} spec @param {unknown} value @returns {Record<string, any>} */
function dereference(spec, value) {
  if (!isObject(value)) return {};
  const resolved = typeof value.$ref === "string"
    ? resolveRef(spec, value.$ref)
    : undefined;
  if (isObject(resolved)) return resolved;
  if (Array.isArray(value.allOf) && value.allOf.length === 1) {
    return dereference(spec, value.allOf[0]);
  }
  return value;
}

/** @param {unknown} schema @returns {string} */
function schemaName(schema) {
  if (!isObject(schema)) return "value";
  if (typeof schema.$ref === "string") return schema.$ref.split("/").at(-1) || "value";
  for (const key of ["allOf", "oneOf", "anyOf"]) {
    if (Array.isArray(schema[key])) return schema[key].map(schemaName).join(" | ");
  }
  if (schema.type === "array") return `${schemaName(schema.items)}[]`;
  if (Array.isArray(schema.type)) {
    return schema.type.filter((value) => typeof value === "string").join(" | ");
  }
  if (typeof schema.type === "string") return schema.type;
  if (Array.isArray(schema.enum)) {
    return schema.enum
      .map((value) => typeof value === "string" ? value : JSON.stringify(value))
      .join(" | ");
  }
  return "value";
}

/** @param {unknown} value @param {number} limit */
function compactText(value, limit) {
  if (typeof value !== "string") return "";
  const text = value.split(/\s+/).filter(Boolean).join(" ");
  const characters = [...text];
  return characters.length <= limit
    ? text
    : `${characters.slice(0, Math.max(0, limit - 1)).join("")}…`;
}

/**
 * @param {Record<string, any>} spec
 * @param {Record<string, any>} shape
 * @param {string} prefix
 * @param {number} depth
 * @returns {{ name: string, type: string, required: boolean, description: string, ownDescription: string }[]}
 */
function schemaFields(spec, shape, prefix = "", depth = 0, context = "") {
  const required = new Set(Array.isArray(shape.required) ? shape.required : []);
  const fields = [];

  for (const [name, raw] of Object.entries(
    isObject(shape.properties) ? shape.properties : {},
  )) {
    const resolved = dereference(spec, raw);
    const path = prefix ? `${prefix}.${name}` : name;
    const ownDescription = compactText(
      isObject(raw) && raw.description !== undefined
        ? raw.description
        : resolved.description,
      MAX_FIELD_DESCRIPTION,
    );
    const description = compactText(
      [context, ownDescription].filter(Boolean).join(". "),
      MAX_FIELD_DESCRIPTION,
    );
    fields.push({
      name: path,
      type: schemaName(raw),
      required: required.has(name),
      description,
      ownDescription,
    });
    if (depth < 1 && isObject(resolved.properties)) {
      fields.push(...schemaFields(spec, resolved, path, depth + 1, description));
    }
    if (fields.length >= MAX_FIELDS) break;
  }
  return fields.slice(0, MAX_FIELDS);
}

/**
 * @param {Record<string, any>} spec
 * @param {unknown} raw
 * @returns {import("./index.js").ApiParameter | undefined}
 */
function parameterDetails(spec, raw) {
  const parameter = dereference(spec, raw);
  if (typeof parameter.name !== "string" || typeof parameter.in !== "string") {
    return undefined;
  }
  const location = parameter.in;
  if (location !== "path" && location !== "query") return undefined;
  const rawSchema = isObject(parameter.schema) ? parameter.schema : {};
  const schema = dereference(spec, rawSchema);
  return {
    name: parameter.name,
    in: location,
    required: location === "path" || parameter.required === true,
    type: schemaName(rawSchema),
    valueType: schemaName(schema),
    ...(typeof schema.format === "string" ? { format: schema.format } : {}),
    ...(Array.isArray(schema.enum) ? { enum: schema.enum.slice(0, 64) } : {}),
    description: compactText(
      parameter.description !== undefined ? parameter.description : schema.description,
      MAX_PARAMETER_DESCRIPTION,
    ),
  };
}

/** @param {Record<string, any>} spec @param {Record<string, any>} operation */
function responseDetails(spec, operation) {
  if (!isObject(operation.responses)) return undefined;
  const raw = operation.responses["200"] ??
    Object.entries(operation.responses)
      .find(([status]) => status.length === 3 && /^2\d\d$/.test(status))?.[1];
  const response = dereference(spec, raw);
  if (!isObject(response.content)) return undefined;
  const content = Object.entries(response.content);
  const selected = content.find(([type]) => type.includes("json")) ??
    content.find(([type]) => type.startsWith("text/"));
  if (!selected) return undefined;

  const [contentType, media] = selected;
  const rawSchema = isObject(media) && isObject(media.schema) ? media.schema : {};
  const schema = dereference(spec, rawSchema);
  const itemSchema = schema.type === "array" ? schema.items : undefined;
  const shape = dereference(spec, itemSchema ?? schema);
  return {
    contentType,
    type: schemaName(rawSchema),
    description: compactText(shape.description, MAX_DESCRIPTION),
    fields: schemaFields(spec, shape),
  };
}

/**
 * Convert BRK's OpenAPI source of truth into the flat read-only operations
 * consumed by the browser search index.
 *
 * @param {unknown} value
 * @returns {import("./index.js").ApiOperation[]}
 */
export function operationsFromOpenApi(value) {
  if (!isObject(value) || !isObject(value.paths)) {
    throw new Error("Unsupported OpenAPI document");
  }
  /** @type {import("./index.js").ApiOperation[]} */
  const operations = [];
  for (const [path, pathItem] of Object.entries(value.paths)) {
    if (!isObject(pathItem) || !isObject(pathItem.get) || pathItem.get.deprecated === true) {
      continue;
    }
    const operation = pathItem.get;
    const response = responseDetails(value, operation);
    if (!response) continue;
    /** @type {import("./index.js").ApiParameter[]} */
    const parameters = [];
    for (const raw of [
      ...(Array.isArray(pathItem.parameters) ? pathItem.parameters : []),
      ...(Array.isArray(operation.parameters) ? operation.parameters : []),
    ]) {
      const parameter = parameterDetails(value, raw);
      if (parameter) parameters.push(parameter);
    }
    const summary = compactText(operation.summary, MAX_DESCRIPTION);
    const key = `GET ${path}`;
    operations.push({
      key,
      method: "GET",
      path,
      label: summary || key,
      summary,
      description: compactText(operation.description, MAX_DESCRIPTION),
      parameters,
      response,
    });
  }
  return operations.sort((left, right) => left.path.localeCompare(right.path));
}
