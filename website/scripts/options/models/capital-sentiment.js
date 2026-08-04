import { brk } from "../../utils/client.js";
import { colors } from "../../utils/colors.js";
import { Unit } from "../../utils/units.js";
import { histogram } from "../series.js";

/**
 * Create Capital Sentiment model section.
 * @returns {PartialOptionsGroup}
 */
export function createCapitalSentimentSection() {
  const { capitalSentiment } = brk.series.models;

  return {
    name: "Capital Sentiment",
    tree: [
      {
        name: "Score",
        title: "Capital Sentiment Score",
        bottom: [
          histogram({
            series: capitalSentiment.score,
            name: "Score",
            unit: Unit.count,
            colorFn: (score) =>
              score === 2
                ? colors.capitalSentiment.bull
                : score === 1
                  ? colors.capitalSentiment.cautiousBull
                  : score === -1
                    ? colors.capitalSentiment.limbo
                    : colors.capitalSentiment.bear,
          }),
        ],
      },
      {
        name: "Position",
        title: "Capital Sentiment Position",
        bottom: [
          histogram({
            series: capitalSentiment.isLong,
            name: "Long",
            unit: Unit.state,
            color: colors.capitalSentiment.bull,
          }),
          histogram({
            series: capitalSentiment.isShort,
            name: "Short",
            unit: Unit.state,
            color: colors.capitalSentiment.bear,
          }),
        ],
      },
    ],
  };
}
