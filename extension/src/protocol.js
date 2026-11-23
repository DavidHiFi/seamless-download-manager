import { api, withTimeout } from './platform.js';
import { load as loadOptions } from './options-store.js';

export const HOST = 'com.seamless.download_manager';
export const DEFAULT_PORT = 15161;

const PING_MS = 500;
const ADD_MS = 2000;
const WAKE_MS = 12000;
const CONFIG_MS = 2000;
const HANDOFF_MS = 10000;
const HOST_RETRY_MS = 60000;

const MSG = {
  stopped: 'Seamless is not running. Open it and try again.',
  noHost: 'The Seamless helper is not installed yet. Open Seamless once so it can set itself up, then try again.',
  key: 'Seamless refused the key. Clear the key box in the extension settings to use the automatic one.',
  slow: 'Seamless did not answer in time. Open it and try again.',
  refused: 'Seamless refused this download.'
};

let cached = null;
let hostMissingUntil = 0;

class ProtocolError extends Error {
  constructor(kind, message) {
    super(message);
    this.kind = kind;
  }
}

function isHostMissing(error) {
  return /not found|forbidden|access to the specified native messaging host/i.test(String(error?.message || error));
}

async function nativeCall(action, payload, ms) {
  if (Date.now() < hostMissingUntil) throw new ProtocolError('no-host', MSG.noHost);
  const id = globalThis.crypto?.randomUUID ? crypto.randomUUID() : String(Date.now() + Math.random());
  let reply;
  try {
    reply = await withTimeout(api.runtime.sendNativeMessage(HOST, { id, action, payload }), ms, 'timeout');
  } catch (error) {
    if (isHostMissing(error)) {
      hostMissingUntil = Date.now() + HOST_RETRY_MS;
      throw new ProtocolError('no-host', MSG.noHost);
    }
    if (String(error?.message) === 'timeout') throw new ProtocolError('slow', MSG.slow);
    throw new ProtocolError('native', MSG.stopped);
  }
  if (!reply || reply.ok !== true) {
    throw new ProtocolError('refused', typeof reply?.error === 'string' && reply.error ? reply.error : MSG.refused);
  }
  return reply.result;
}

async function readSessionConn() {
  try {
    const stored = await api.storage.session?.get('conn');
    return stored?.conn ?? null;
  } catch {
    return null;
  }
}

async function fetchConfig() {
  const result = await nativeCall('config', {}, CONFIG_MS);
  const port = Number(result?.port);
  const apiKey = typeof result?.apiKey === 'string' ? result.apiKey : '';
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new ProtocolError('refused', MSG.refused);
  cached = { port, apiKey };
  try {
    await api.storage.session?.set({ conn: cached });
  } catch {}
  return cached;
}

async function autoConn(refresh) {
  if (!refresh) {
    cached = cached ?? (await readSessionConn());
    if (cached) return cached;
  }
  try {
    return await fetchConfig();
  } catch {
    return null;
  }
}

function hasOverride(options) {
  return Boolean(options.manualPort || options.manualKey);
}

async function resolveConn(options, refresh = false) {
  const auto = hasOverride(options) && options.manualPort && options.manualKey ? null : await autoConn(refresh);
  const port = options.manualPort ?? auto?.port ?? (hasOverride(options) ? DEFAULT_PORT : null);
  if (!port) return null;
  return { port, apiKey: options.manualKey || auto?.apiKey || '' };
}

async function http(conn, method, path, body, ms) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), ms);
  let response;
  try {
    response = await fetch(`http://127.0.0.1:${conn.port}${path}`, {
      method,
      headers: {
        ...(conn.apiKey ? { 'X-Api-Key': conn.apiKey } : {}),
        ...(body === undefined ? {} : { 'Content-Type': 'application/json' })
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: controller.signal,
      cache: 'no-store'
    });
  } catch (error) {
    throw new ProtocolError(error?.name === 'AbortError' ? 'slow' : 'down', error?.name === 'AbortError' ? MSG.slow : MSG.stopped);
  } finally {
    clearTimeout(timer);
  }
  let json = null;
  try {
    json = await response.json();
  } catch {}
  if (response.status === 401 || response.status === 403) throw new ProtocolError('key', MSG.key);
  if (!response.ok || json?.ok === false) {
    throw new ProtocolError('refused', typeof json?.error === 'string' && json.error ? json.error : MSG.refused);
  }
  return json;
}

async function httpWithRefresh(options, method, path, body, ms) {
  let conn = await resolveConn(options);
  if (!conn) throw new ProtocolError('no-host', MSG.noHost);
  try {
    return await http(conn, method, path, body, ms);
  } catch (error) {
    if (!['down', 'key'].includes(error.kind) || (options.manualPort && options.manualKey)) throw error;
    const fresh = await resolveConn(options, true);
    if (!fresh || (fresh.port === conn.port && fresh.apiKey === conn.apiKey)) throw error;
    conn = fresh;
    return http(conn, method, path, body, ms);
  }
}

function failure(error) {
  return { ok: false, error: error?.message || MSG.stopped, kind: error?.kind || 'down' };
}

export async function status() {
  const options = await loadOptions();
  try {
    const reply = await httpWithRefresh(options, 'POST', '/ping', undefined, PING_MS);
    return { ok: true, state: 'connected', version: reply?.version ?? '' };
  } catch (error) {
    if (error.kind === 'no-host') return { ok: false, state: 'no-host', error: error.message };
    if (error.kind === 'key') return { ok: false, state: 'key', error: error.message };
    return { ok: false, state: 'stopped', error: 'Seamless is not running. Open it, or start a download and it opens by itself.' };
  }
}

export async function test({ port, key } = {}) {
  const options = await loadOptions();
  const merged = {
    ...options,
    manualPort: port === undefined ? options.manualPort : port,
    manualKey: key === undefined ? options.manualKey : key
  };
  let conn;
  try {
    conn = await resolveConn(merged, true);
  } catch {
    conn = null;
  }
  if (!conn) return { ok: false, message: MSG.noHost };
  try {
    const reply = await http(conn, 'POST', '/ping', undefined, 1500);
    const version = reply?.version ? ` (version ${reply.version})` : '';
    return { ok: true, message: `Connected on port ${conn.port}${version}.` };
  } catch (error) {
    if (error.kind === 'key') return { ok: false, message: error.message };
    if (error.kind === 'slow') return { ok: false, message: `Port ${conn.port} did not answer in time. Check that Seamless is running.` };
    return { ok: false, message: `Nothing answered on port ${conn.port}. Open Seamless, or check the port in its settings.` };
  }
}

export async function add(items, flags) {
  const options = await loadOptions();
  const body = { items, options: { silentAdd: Boolean(flags.silentAdd), silentStart: Boolean(flags.silentAdd && flags.silentStart) } };
  let nativeError = null;
  if (!hasOverride(options)) {
    try {
      await nativeCall('add', body, WAKE_MS);
      return { ok: true, via: 'native' };
    } catch (error) {
      nativeError = error;
      if (error.kind === 'refused') return failure(error);
    }
  }
  try {
    await httpWithRefresh(options, 'POST', '/add', body, ADD_MS);
    return { ok: true, via: 'http' };
  } catch (error) {
    return failure(nativeError && nativeError.kind !== 'no-host' && error.kind === 'down' ? nativeError : error);
  }
}

export async function handoff(payload) {
  const options = await loadOptions();
  let nativeError = null;
  if (!hasOverride(options)) {
    try {
      await nativeCall('handoff', payload, HANDOFF_MS);
      return { ok: true, via: 'native' };
    } catch (error) {
      nativeError = error;
      if (error.kind === 'refused') return failure(error);
    }
  }
  try {
    await httpWithRefresh(options, 'POST', '/v1/handoff', payload, HANDOFF_MS);
    return { ok: true, via: 'http' };
  } catch (error) {
    return failure(nativeError && nativeError.kind !== 'no-host' && error.kind === 'down' ? nativeError : error);
  }
}
