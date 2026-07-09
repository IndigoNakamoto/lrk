/**
 * @param {Element} element
 * @param {string} property
 */
export function transitionMs(element, property) {
  const style = getComputedStyle(element);
  const properties = style.transitionProperty.split(",").map((part) => {
    return part.trim();
  });
  const durations = parseCssTimes(style.transitionDuration);
  const delays = parseCssTimes(style.transitionDelay);
  const index = properties.findIndex((part) => {
    return part === property || part === "all";
  });

  if (index < 0) return 0;

  const duration = durations[index] ?? durations.at(-1) ?? 0;
  const delay = delays[index] ?? delays.at(-1) ?? 0;

  return duration + delay;
}

/** @param {string} value */
function parseCssTimes(value) {
  return value.split(",").map((part) => {
    const time = part.trim();
    const amount = Number.parseFloat(time);

    return time.endsWith("ms") ? amount : amount * 1_000;
  });
}
