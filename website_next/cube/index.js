import { CUBE_FACE_SPECS, createCubeFace } from "./faces.js";

/**
 * @param {{ fill?: number }} [options]
 */
export function createCube({ fill = 0.5 } = {}) {
  const cube = document.createElement("span");
  cube.dataset.cube = "mark";
  cube.setAttribute("aria-hidden", "true");
  cube.style.setProperty("--fill", String(fill));
  cube.append(
    ...CUBE_FACE_SPECS.map(([role, side]) =>
      createCubeFace("span", role, side),
    ),
  );

  return cube;
}
