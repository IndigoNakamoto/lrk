/** @param {string} name */
export function getPoolSlug(name) {
  return name.toLowerCase().replace(/[^a-z0-9]/g, "");
}

/** @param {string} name */
export function getPoolDisplayName(name) {
  return name.replace(/\s+(Pool|USA)$/i, "").trim();
}

/** @param {{ name: string, slug?: string }} pool */
export function createPoolLogo(pool) {
  const logo = document.createElement("img");
  const slug = pool.slug || getPoolSlug(pool.name);

  logo.src = `/assets/pools/${slug}.svg`;
  logo.alt = "";
  logo.onerror = () => {
    logo.onerror = null;
    logo.src = "/assets/pools/default.svg";
  };

  return logo;
}
