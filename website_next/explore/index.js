import { createBlockDetails } from "./block/index.js";
import { createChain } from "./chain/index.js";

export function createExplorePage() {
  const main = document.createElement("main");
  const blockDetails = createBlockDetails();
  const chain = createChain({
    onSelect: blockDetails.update,
  });

  main.dataset.page = "explore";
  main.append(chain.element, blockDetails.element);

  const syncChain = () => chain.setActive(!main.hidden && !document.hidden);

  main.addEventListener("pageactive", syncChain);
  document.addEventListener("visibilitychange", syncChain);
  new MutationObserver(syncChain).observe(main, {
    attributes: true,
    attributeFilter: ["hidden"],
  });

  return main;
}
