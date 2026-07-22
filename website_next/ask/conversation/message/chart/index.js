import { createChart } from "../../../../chart/index.js";
import { units } from "../../../../chart/units.js";
import { colors } from "../../../../utils/colors.js";
import { createMetric } from "../../../tools/metrics/index.js";

/** @typedef {import("../../../storage.js").ChartArtifact} ChartArtifact */

/** @param {ChartArtifact} artifact */
export function createAskChart(artifact) {
  const section = document.createElement("section");
  const spec = artifact.chart;

  section.dataset.askChart = "";
  try {
    const chart = {
      title: spec.title,
      unit: units[spec.unit],
      defaultType: spec.view,
      defaultScale: spec.scale,
      series: spec.series.map((item) => ({
        label: item.label,
        color: colors[item.color],
        metric: createMetric(item.path),
      })),
    };
    section.append(createChart(chart, `ask-${artifact.id}`));
  } catch {
    section.dataset.state = "unavailable";
    section.textContent = "Chart unavailable";
  }
  return section;
}
