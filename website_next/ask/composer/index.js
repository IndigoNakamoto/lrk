/**
 * @typedef {Object} AskComposerOptions
 * @property {(question: string) => void} onSubmit
 * @property {() => void} onStop
 */

/** @param {AskComposerOptions} options */
export function createAskComposer(options) {
  const form = document.createElement("form");
  const fieldset = document.createElement("fieldset");
  const label = document.createElement("label");
  const labelText = document.createElement("span");
  const input = document.createElement("textarea");
  const actions = document.createElement("div");
  const send = document.createElement("button");
  const sendIcon = document.createElement("span");
  const stop = document.createElement("button");

  form.dataset.askComposer = "";
  form.hidden = true;
  fieldset.disabled = true;
  labelText.append("Message assistant");
  input.name = "question";
  input.rows = 1;
  input.maxLength = 4_000;
  input.placeholder = "Ask anything";
  input.required = true;
  send.type = "submit";
  send.ariaLabel = "Send message";
  send.title = "Send message";
  sendIcon.ariaHidden = "true";
  sendIcon.append("↑");
  send.append(sendIcon);
  stop.type = "button";
  stop.ariaLabel = "Stop generating";
  stop.title = "Stop generating";
  stop.hidden = true;
  stop.addEventListener("click", options.onStop);
  input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" || event.shiftKey) return;

    event.preventDefault();
    form.requestSubmit();
  });
  form.addEventListener("submit", (event) => {
    event.preventDefault();

    const question = input.value.trim();
    if (question) options.onSubmit(question);
  });
  label.append(labelText, input);
  actions.append(send, stop);
  fieldset.append(label, actions);
  form.append(fieldset);

  return {
    element: form,
    get value() {
      return input.value;
    },
    set value(value) {
      input.value = value;
    },
    focus() {
      input.focus();
    },
    hide() {
      form.hidden = true;
      fieldset.disabled = true;
    },
    ready() {
      form.hidden = false;
      fieldset.disabled = false;
      input.disabled = false;
      send.hidden = false;
      stop.hidden = true;
    },
    generating() {
      form.hidden = false;
      fieldset.disabled = false;
      input.disabled = true;
      send.hidden = true;
      stop.hidden = false;
    },
  };
}
