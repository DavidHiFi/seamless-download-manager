export const ALLOWED = Object.freeze([
  'Cookie', 'Referer', 'User-Agent', 'Authorization', 'Accept-Language', 'Origin'
]);

const CANONICAL = new Map(ALLOWED.map((name) => [name.toLowerCase(), name]));

export function filterHeaders(list) {
  const out = {};
  for (const header of list || []) {
    if (!header || typeof header.name !== 'string') continue;
    const name = CANONICAL.get(header.name.toLowerCase());
    if (!name) continue;
    const value = header.value ?? '';
    if (value === '') continue;
    out[name] = String(value);
  }
  return out;
}

export function cookieHeader(cookies) {
  if (!Array.isArray(cookies) || cookies.length === 0) return '';
  return cookies
    .slice()
    .sort((a, b) => (b.path?.length || 0) - (a.path?.length || 0))
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
}

export function buildHeaders({ sendHeaders, wire, cookies, referer, userAgent }) {
  if (!sendHeaders) return null;
  const out = Array.isArray(wire) ? filterHeaders(wire) : { ...(wire || {}) };
  if (!out.Cookie) {
    const cookie = cookieHeader(cookies);
    if (cookie) out.Cookie = cookie;
  }
  if (!out.Referer && referer) out.Referer = referer;
  if (!out['User-Agent'] && userAgent) out['User-Agent'] = userAgent;
  return Object.keys(out).length ? out : null;
}

export function responseHeader(list, name) {
  const wanted = name.toLowerCase();
  for (const header of list || []) {
    if (header && typeof header.name === 'string' && header.name.toLowerCase() === wanted) {
      return header.value ?? '';
    }
  }
  return '';
}
