import { createBuildPage } from "./build/index.js";
import { createExplorePage } from "./explore/index.js";
import { createHomePage } from "./home/index.js";
import { createLearnPage } from "./learn/index.js";
import { createWalletsPage } from "./wallets/index.js";

const routes = /** @type {const} */ ({
  "/": createHomePage,
  "/explore": createExplorePage,
  "/learn": createLearnPage,
  "/build": createBuildPage,
  "/wallets": createWalletsPage,
});

/** @typedef {keyof typeof routes} RoutePath */

/** @param {string} pathname */
function canonicalPath(pathname) {
  return pathname !== "/" && pathname.endsWith("/")
    ? pathname.slice(0, -1)
    : pathname;
}

/**
 * @param {string} pathname
 * @returns {RoutePath | undefined}
 */
export function resolvePath(pathname) {
  const path = canonicalPath(pathname);

  return path in routes ? /** @type {RoutePath} */ (path) : undefined;
}

/**
 * @param {string} pathname
 * @returns {RoutePath}
 */
export function normalizePath(pathname) {
  return resolvePath(pathname) ?? "/";
}

/** @param {RoutePath} pathname */
export function createRoutePage(pathname) {
  return routes[pathname]();
}
