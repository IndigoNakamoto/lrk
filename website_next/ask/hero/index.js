const SUGGESTIONS = /** @type {const} */ ([
  ["Explain a metric", "Explain this Bitcoin metric simply: "],
  ["Explore mining", "Explain Bitcoin mining and proof of work."],
  ["Understand fees", "Explain how Bitcoin transaction fees work."],
]);

/** @param {{ onPrompt: (prompt: string) => void }} options */
export function createAskHero(options) {
  const hero = document.createElement("div");
  const eyebrow = document.createElement("p");
  const title = document.createElement("h1");
  const description = document.createElement("p");
  const suggestions = document.createElement("nav");

  hero.dataset.askHero = "";
  eyebrow.append("Bitcoin · Private · On-device");
  title.append("Ask about Bitcoin");
  description.append(
    "Explore the protocol, network, and on-chain data—privately, in your browser.",
  );
  suggestions.setAttribute("aria-label", "Suggested questions");

  for (const [label, prompt] of SUGGESTIONS) {
    const button = document.createElement("button");
    const arrow = document.createElement("span");

    button.type = "button";
    arrow.ariaHidden = "true";
    arrow.append("↗");
    button.append(label, arrow);
    button.addEventListener("click", () => options.onPrompt(prompt));
    suggestions.append(button);
  }

  hero.append(eyebrow, title, description, suggestions);

  return {
    element: hero,
    /** @param {boolean} enabled */
    setEnabled(enabled) {
      for (const button of suggestions.querySelectorAll("button")) {
        button.disabled = !enabled;
      }
    },
  };
}
