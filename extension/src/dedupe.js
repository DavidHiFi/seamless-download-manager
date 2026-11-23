export const WINDOW_MS = 15000;
export const FAILED_MS = 4000;

export function normalizeUrl(url) {
  if (typeof url !== 'string') return '';
  const hash = url.indexOf('#');
  return hash === -1 ? url : url.slice(0, hash);
}

export function keysFor({ chain = [], requestId, downloadId } = {}) {
  const keys = new Set();
  for (const url of chain) {
    const n = normalizeUrl(url);
    if (n) keys.add(`u:${n}`);
  }
  if (requestId !== undefined && requestId !== null) keys.add(`r:${requestId}`);
  if (downloadId !== undefined && downloadId !== null) keys.add(`d:${downloadId}`);
  return [...keys];
}

export function createDeduper({ now = Date.now, store = {}, windowMs = WINDOW_MS, failedMs = FAILED_MS } = {}) {
  function expired(entry, t) {
    return t - entry.t > (entry.s === 'failed' ? failedMs : windowMs);
  }

  function prune() {
    const t = now();
    for (const key of Object.keys(store)) if (expired(store[key], t)) delete store[key];
  }

  function status(keys) {
    prune();
    let found = null;
    for (const key of keys) {
      const entry = store[key];
      if (!entry) continue;
      if (entry.s === 'accepted') return 'accepted';
      found = entry.s;
    }
    return found;
  }

  function claim(keys) {
    const seen = status(keys);
    if (seen) return { ok: false, status: seen };
    const t = now();
    for (const key of keys) store[key] = { t, s: 'sending' };
    return { ok: true };
  }

  function settle(keys, state) {
    const t = now();
    for (const key of keys) store[key] = { t, s: state };
  }

  function release(keys) {
    for (const key of keys) delete store[key];
  }

  return { claim, settle, release, status, prune, store };
}
