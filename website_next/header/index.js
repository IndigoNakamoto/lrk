import { createBrand } from "../brand/index.js";

export function createHeader() {
  const header = document.createElement("header");

  header.append(createBrand({ href: "/" }));
  return header;
}
