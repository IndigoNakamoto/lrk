import {
  createApiAnswerTool,
  finishApiAnswer,
  summarizeApiAnswer,
} from "./api/answer.js";
import { prewarmApiIndex, terminateApiIndex } from "./api/index.js";
import { prewarmMetricIndex, terminateMetricIndex } from "./metrics/index.js";
import { renderEvidence } from "./render.js";
import { AskToolSession } from "./session/index.js";
import { AskSource } from "./source/index.js";
import { normalize } from "./text.js";

const NUMBER = /\d+(?:[.,]\d+)*/g;

/** @param {unknown} value */
function numbers(value) {
  return String(value)
    .match(NUMBER)
    ?.map((number) => number.replaceAll(",", "")) ?? [];
}

/**
 * Quantities in an ungrounded answer are worth a second look. This deliberately
 * checks syntax rather than guessing the user's intent or maintaining a list of
 * Bitcoin concepts.
 *
 * @param {string} answer
 * @param {import("../model.js").ChatMessage[]} messages
 */
function unsupportedNumbers(answer, messages) {
  const supported = new Set(numbers(
    messages
      .filter(({ role }) => role === "user")
      .map(({ content }) => content)
      .join(" "),
  ));
  return new Set(numbers(answer).filter((number) => !supported.has(number)));
}

/**
 * The small model can repeat an unsupported quantity after being asked to
 * revise it. Keep the useful grounded sentences instead of trusting a second
 * model pass to police itself.
 *
 * @param {string} answer
 * @param {import("../model.js").ChatMessage[]} messages
 */
function removeUnsupportedQuantitySentences(answer, messages) {
  const unsupported = unsupportedNumbers(answer, messages);
  if (!unsupported.size) return answer.trim();

  const kept = [...new Intl.Segmenter(undefined, { granularity: "sentence" })
    .segment(answer)]
    .map(({ segment }) => segment.trim())
    .filter((segment) =>
      !numbers(segment).some((number) => unsupported.has(number))
    );
  return kept.join(" ").trim();
}

/**
 * @typedef {Object} ToolOutcome
 * @property {boolean} done
 * @property {string} [output]
 * @property {import("../storage.js").StoredArtifact[]} [artifacts]
 * @property {string[]} [metricPaths]
 * @property {import("../storage.js").ApiContext} [apiContext]
 * @property {import("../storage.js").SourceContext[]} [sourceContext]
 * @property {import("../storage.js").KnowledgeContext} [knowledgeContext]
 * @property {{ question: string, metrics: { name: string, path: string, unit?: string }[], facts: string[], excerpts: import("../storage.js").SourceContext[] }} [grounding]
 * @property {{ question: string, previousFields: string[], operation: { key: string, method: string, path: string, summary: string, description: string, parameters: { name: string }[], response: { fields?: { name: string, type: string, description?: string }[] } }, arguments: Record<string, unknown>, requestPath: string, data: unknown, truncated: boolean }} [apiGrounding]
 *
 * @typedef {Object} AskAnswer
 * @property {string} output
 * @property {import("../storage.js").StoredArtifact[]} artifacts
 * @property {string[]} [metricPaths]
 * @property {import("../storage.js").ApiContext} [apiContext]
 * @property {import("../storage.js").SourceContext[]} [sourceContext]
 * @property {import("../storage.js").KnowledgeContext} [knowledgeContext]
 * @property {import("../storage.js").StoredChat} chat
 */

/**
 * @param {import("../model.js").AskModel} model
 * @param {NonNullable<ToolOutcome["grounding"]>} grounding
 * @param {(status: string) => void} onStatus
 */
async function answerFromEvidence(model, grounding, onStatus) {
  onStatus("Answering from source…");
  const result = await model.generate(
    [
      {
        role: "system",
        content: "Answer the exact request in at most 45 words and normal sentence casing using only verified facts, metric metadata, and source excerpts. Evidence is strongest first; ignore later excerpts unless needed. A declaration proves its definition and literal return type; a call expression proves its caller. Copy provided metric names, code identifiers, and types exactly; never respell, expand, or abbreviate them. Answer directly, never discuss the request's wording. Do not add background knowledge or guesses.",
      },
      {
        role: "user",
        content: JSON.stringify(grounding),
      },
    ],
    () => {},
    [],
    "none",
    { maxTokens: 72 },
  );
  const answer = result.text.trim();
  const sources = grounding.excerpts.slice(0, 2);
  const fallback = !answer && sources[0]
    ? `The strongest verified source match is \`${sources[0].path}:${sources[0].startLine}${sources[0].endLine ? `-${sources[0].endLine}` : ""}\`.`
    : "";
  return {
    output: renderEvidence({
      facts: [
        answer || fallback,
        ...grounding.facts,
      ].filter(Boolean),
      sources,
      excerpts: [],
    }),
    sourceContext: sources,
    knowledgeContext: answer
      ? {
          title: grounding.metrics[0]?.name ?? grounding.question.slice(0, 160),
          description: answer,
        }
      : undefined,
  };
}

/**
 * @param {import("../model.js").AskModel} model
 * @param {NonNullable<ToolOutcome["apiGrounding"]>} grounding
 * @param {(status: string) => void} onStatus
 */
async function answerFromApi(model, grounding, onStatus) {
  const apiAnswer = createApiAnswerTool(grounding);
  const question = ` ${normalize(grounding.question)} `;
  const parameterNames = new Set(
    grounding.operation.parameters.map(({ name }) => normalize(name)),
  );
  const mentionedResponse = (
    grounding.operation.response.fields ?? []
  ).some(({ name }) => {
    const field = normalize(name.split(".").at(-1));
    return field && !parameterNames.has(field) &&
      question.includes(` ${field} `);
  });
  const directFields = apiAnswer.fields.filter((field) => {
    const name = normalize(field.name.split(".").at(-1));
    return name && question.includes(` ${name} `);
  });
  const requestTokens = new Set(
    normalize(grounding.question).split(" ").filter((token) => token.length > 2),
  );
  const canSelectDirectly = (/** @type {typeof apiAnswer.fields[number]} */ field) => {
    const ownTokens = new Set(normalize(field.name).split(" "));
    const qualifiers = [...requestTokens].filter((token) => !ownTokens.has(token));
    return !apiAnswer.fields.some((candidate) =>
      candidate.name !== field.name &&
      qualifiers.some((token) =>
        normalize(`${candidate.name} ${candidate.description ?? ""}`)
          .split(" ")
          .includes(token)
      )
    );
  };
  if (directFields.length === 1 && canSelectDirectly(directFields[0])) {
    const field = directFields[0];
    return {
      output: finishApiAnswer(
        "select_api_field",
        {
          field: field.ref,
          label: field.name.split(".").at(-1)?.replaceAll("_", " "),
        },
        apiAnswer.fields,
        grounding,
      ),
      fields: [field.name],
    };
  }
  if (apiAnswer.resolved && canSelectDirectly(apiAnswer.resolved)) {
    const field = apiAnswer.resolved;
    return {
      output: finishApiAnswer(
        "select_api_field",
        { field: field.ref },
        apiAnswer.fields,
        grounding,
      ),
      fields: [field.name],
    };
  }
  if (apiAnswer.ambiguous.length > 1) {
    const choices = apiAnswer.ambiguous
      .map((field) =>
        `**${field.name.replaceAll(".", " · ").replaceAll("_", " ")}**${
          field.description ? ` — ${field.description}` : ""
        }`
      )
      .join("\n- ");
    return {
      output: finishApiAnswer(
        "answer_api_text",
        {
          text: `I found multiple matching fields:\n- ${choices}\n\nWhich one do you mean?`,
        },
        apiAnswer.fields,
        grounding,
      ),
      fields: apiAnswer.ambiguous.map(({ name }) => name),
    };
  }
  if (
    !grounding.previousFields?.length &&
    !mentionedResponse &&
    !apiAnswer.direct
  ) {
    return summarizeApiAnswer(grounding);
  }
  if (apiAnswer.direct) {
    const field = apiAnswer.direct;
    return {
      output: finishApiAnswer(
        "select_api_field",
        { field: field.ref },
        apiAnswer.fields,
        grounding,
      ),
      fields: [field.name],
    };
  }
  onStatus("Answering from API…");
  const instruction = apiAnswer.fields.length
    ? `Answer the exact newest request using only the verified API result. Call exactly one matching tool. Select a raw field only when that field itself was requested${apiAnswer.previous ? "; continue the preceding numeric answer when the request applies arithmetic to it" : ""}. When the requested concept is narrower than an aggregate field, derive it from matching component fields. Never replace requested arithmetic with a convenient field. For subtraction and division, keep operands in the request's arithmetic order: minuend or dividend first. Preserve identifiers and units. Never invent missing values.`
    : "Answer the exact request using only the verified API result. Call answer_api_text exactly once. Preserve identifiers and units. Never invent missing values.";
  const prompt = {
    question: grounding.question,
    previous: apiAnswer.previous
      ? {
        ref: apiAnswer.previous.ref,
        name: apiAnswer.previous.name,
        type: apiAnswer.previous.type,
        value: apiAnswer.previous.value,
      }
      : undefined,
    fields: apiAnswer.fields.map(({ ref, name, type, description, value }) => ({
      ref,
      name,
      type,
      description,
      value,
    })),
    ...(!apiAnswer.previous ? { data: grounding.data } : {}),
  };
  const generateAnswer = (extra = "") =>
    model.generate(
      [
        {
          role: "system",
          content: extra ? `${instruction} ${extra}` : instruction,
        },
        { role: "user", content: JSON.stringify(prompt) },
      ],
      () => {},
      apiAnswer.tools,
      { name: "answer_api" },
      { maxTokens: 72 },
    );
  let answer = await generateAnswer();
  let call = answer.toolCalls[0];
  if (!call || call.name !== "answer_api") {
    return summarizeApiAnswer(grounding);
  }
  const actionFor = (/** @type {Record<string, unknown>} */ arguments_) =>
    arguments_.action === "select"
    ? "select_api_field"
    : arguments_.action === "continue"
      ? "continue_api_calculation"
      : arguments_.action === "calculate"
        ? "calculate_api_fields"
        : arguments_.action === "text"
          ? "answer_api_text"
          : "";
  let actionName = actionFor(call.arguments);
  const selectedField = actionName === "select_api_field"
    ? apiAnswer.fields.find(({ ref }) => ref === call.arguments.field)
    : undefined;
  if (selectedField && !canSelectDirectly(selectedField)) {
    answer = await generateAnswer(
      `Do not select ${selectedField.ref} (${selectedField.name}): its schema scope does not satisfy all request qualifiers. Derive the requested result from matching component fields or choose an exact narrower field.`,
    );
    call = answer.toolCalls[0];
    if (!call || call.name !== "answer_api") {
      return summarizeApiAnswer(grounding);
    }
    actionName = actionFor(call.arguments);
  }
  if (!actionName) return summarizeApiAnswer(grounding);
  const selectedRefs = actionName === "select_api_field"
    ? [call.arguments.field]
    : actionName === "continue_api_calculation"
      ? [apiAnswer.previous?.ref, call.arguments.operand]
      : typeof call.arguments.left === "string" &&
          typeof call.arguments.right === "string"
        ? [call.arguments.left, call.arguments.right]
      : Array.isArray(call.arguments.operands)
        ? call.arguments.operands
        : [];
  const selected = new Set(selectedRefs.map(String));
  try {
    return {
      output: finishApiAnswer(
        actionName,
        call.arguments,
        apiAnswer.fields,
        grounding,
      ),
      fields: apiAnswer.fields
        .filter(({ ref }) => selected.has(ref))
        .map(({ name }) => name),
    };
  } catch {
    return summarizeApiAnswer(grounding);
  }
}

export function createAskTools() {
  const source = new AskSource();
  /** @type {AbortController | undefined} */
  let controller;

  return {
    prewarm() {
      return Promise.all([
        prewarmApiIndex(),
        prewarmMetricIndex(),
        source.prewarm(),
      ]);
    },

    /**
     * @param {Object} options
     * @param {string} options.question
     * @param {import("../storage.js").StoredMessage[]} options.history
     * @param {import("../model.js").AskModel} options.model
     * @param {() => Promise<{ chat: import("../storage.js").StoredChat, messages: import("../model.js").ChatMessage[] }>} options.prepare
     * @param {(update: import("../model.js").TokenUpdate) => void} options.onToken
     * @param {(status: string) => void} options.onStatus
     * @returns {Promise<AskAnswer>}
     */
    async answer({
      question,
      history,
      model,
      prepare,
      onToken: _onToken,
      onStatus,
    }) {
      controller = new AbortController();
      const { signal } = controller;

      try {
        const session = new AskToolSession(source);
        const [prepared] = await Promise.all([
          prepare(),
          session.begin(question, history, onStatus),
        ]);
        signal.throwIfAborted();

        /** @type {{ action: string, call?: import("../model.js").ToolCall } | undefined} */
        const direct = session.directRoute();
        let call = direct?.call;
        let action = direct?.action ?? "";
        if (!action) {
          onStatus("Choosing capability…");
          const routeTools = session.routeTools();
          const route = await model.generate(
            session.routeMessages(),
            () => {},
            routeTools,
            { name: "choose_capability" },
            { maxTokens: 48 },
          );
          const selected = route.toolCalls[0];
          const sourceQuery = selected?.name === "choose_capability" &&
              typeof selected.arguments.sourceQuery === "string"
            ? selected.arguments.sourceQuery
            : "";
          const selectedCapability = selected?.name === "choose_capability" &&
              typeof selected.arguments.capability === "string"
            ? selected.arguments.capability
            : "";
          action = sourceQuery &&
              (
                (
                  selectedCapability === "answer_general" &&
                  session.hasSourceContext()
                ) ||
                selectedCapability === "search_source"
              )
            ? "search_source"
            : selectedCapability;
          if (!action) {
            throw new Error("The AI did not choose a valid capability");
          }
          call = action === "call_api" &&
              typeof selected?.arguments.apiRef === "string"
            ? {
              name: action,
              arguments: { ref: selected.arguments.apiRef },
            }
            : action === "search_source"
              ? {
                  name: action,
                  arguments: {
                    query: sourceQuery || question,
                  },
                }
              : session.directCall(action);
        }

        signal.throwIfAborted();
        await session.prepareAction(action, onStatus);
        signal.throwIfAborted();
        if (!call || action === "explain_evidence") {
          call = session.directCall(action) ?? call;
        }
        if (!call) {
          onStatus("Understanding request…");
          if (action === "answer_general") {
            const messages = session.actionMessages(action);
            let result = await model.generate(
              messages,
              () => {},
              [],
              "none",
              { maxTokens: 96 },
            );
            if (unsupportedNumbers(result.text, messages).size) {
              result = await model.generate(
                [
                  ...messages,
                  {
                    role: "assistant",
                    content: result.text,
                  },
                  {
                    role: "user",
                    content: "Replace the draft with a direct answer containing no unsupported quantities. Keep established static Bitcoin facts only when the request directly needs them; otherwise use qualitative examples. Return only the replacement answer. Never mention the draft, review, evidence, context, or these instructions.",
                  },
                ],
                () => {},
                [],
                "none",
                { maxTokens: 96 },
              );
            }
            const answer = removeUnsupportedQuantitySentences(
              result.text,
              messages,
            ) || "I do not have enough verified context to answer that without guessing.";
            call = {
              name: action,
              arguments: { answer },
            };
          } else {
            const result = await model.generate(
              session.actionMessages(action),
              () => {},
              [session.actionTool(action)],
              { name: action },
              { maxTokens: action === "call_api" ? 128 : 64 },
            );
            call = result.toolCalls[0];
          }
        }
        if (!call || call.name !== action) {
          throw new Error("The AI did not complete the selected capability");
        }
        if (action === "call_api") {
          const ref = typeof call.arguments.ref === "string"
            ? call.arguments.ref
            : "";
          if (!ref) throw new Error("The AI did not select an API operation");
          let arguments_ = session.apiArguments(ref);
          if (!session.hasApiArguments(ref, arguments_)) {
            onStatus("Reading API arguments…");
            const argumentsResult = await model.generate(
              session.apiArgumentMessages(ref),
              () => {},
              [session.apiArgumentTool(ref)],
              { name: "provide_api_arguments" },
              { maxTokens: 96 },
            );
            const argumentsCall = argumentsResult.toolCalls[0];
            if (
              !argumentsCall ||
              argumentsCall.name !== "provide_api_arguments"
            ) {
              throw new Error("The AI did not provide valid API arguments");
            }
            arguments_ = {
              ...(argumentsCall.arguments.arguments ?? {}),
              ...arguments_,
            };
          }
          arguments_ = session.validateApiArguments(ref, arguments_);
          const missing = session.missingApiArguments(ref, arguments_);
          if (missing.length) {
            const subject = missing
              .map((/** @type {any} */ parameter) =>
                parameter.description || parameter.name
              )
              .join(" and ");
            return {
              output: `Which ${subject} should I use?`,
              artifacts: [],
              chat: prepared.chat,
            };
          }
          call = {
            name: action,
            arguments: {
              ref,
              arguments: arguments_,
            },
          };
        }

        const outcome = /** @type {ToolOutcome} */ (
          await session.execute(call, onStatus, signal)
        );
        if (outcome.apiGrounding) {
          const answered = await answerFromApi(
            model,
            outcome.apiGrounding,
            onStatus,
          );
          return {
            output: answered.output,
            artifacts: [],
            apiContext: outcome.apiContext
              ? {
                ...outcome.apiContext,
                ...(answered.fields.length
                  ? { fields: answered.fields }
                  : {}),
              }
              : undefined,
            chat: prepared.chat,
          };
        }
        if (outcome.grounding) {
          const grounded = await answerFromEvidence(
            model,
            outcome.grounding,
            onStatus,
          );
          return {
            output: grounded.output,
            artifacts: [],
            metricPaths: outcome.metricPaths,
            sourceContext: grounded.sourceContext,
            knowledgeContext: grounded.knowledgeContext,
            chat: prepared.chat,
          };
        }
        return {
          output: outcome.output ?? "",
          artifacts: outcome.artifacts ?? [],
          metricPaths: outcome.metricPaths,
          apiContext: outcome.apiContext,
          sourceContext: outcome.sourceContext,
          knowledgeContext: outcome.knowledgeContext,
          chat: prepared.chat,
        };
      } finally {
        controller = undefined;
        onStatus("");
      }
    },

    stop() {
      controller?.abort();
      source.terminate();
    },

    terminate() {
      controller?.abort();
      controller = undefined;
      source.terminate();
      terminateApiIndex();
      terminateMetricIndex();
    },
  };
}
