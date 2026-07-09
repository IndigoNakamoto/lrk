const localDomains = new Set([
  "localhost",
  "127.0.0.1",
  "0.0.0.0",
  "::1",
  "[::1]",
]);

/**
 * @param {string} domain
 */
function isIpv4Octet(domain) {
  return /^\d+$/.test(domain);
}

/**
 * @param {string} domain
 */
function isPrivateIpv4(domain) {
  const parts = domain.split(".");

  if (
    parts.length !== 4 ||
    parts.some((part) => !isIpv4Octet(part) || Number(part) > 255)
  ) {
    return false;
  }

  const [a, b] = parts.map(Number);

  return (
    a === 10 ||
    a === 127 ||
    (a === 169 && b === 254) ||
    (a === 172 && b >= 16 && b <= 31) ||
    (a === 192 && b === 168)
  );
}

/**
 * @param {{ domain: string }} client
 */
export function isLocalClient(client) {
  const domain = client.domain.toLowerCase();

  return (
    localDomains.has(domain) ||
    domain.endsWith(".local") ||
    isPrivateIpv4(domain)
  );
}
