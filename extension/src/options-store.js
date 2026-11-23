import { api } from './platform.js';

export const STORAGE_KEY = 'options';

export const NEVER_TYPES = Object.freeze(['html', 'htm', 'php', 'xhtml', 'asp', 'aspx', 'jsp']);

export const DEFAULT_INCLUDE_TYPES = Object.freeze([
  'zip', 'rar', '7z', 'iso', 'tar', 'gz', 'bz2', 'xz', 'zst', 'exe', 'msi', 'msix', 'appx',
  'deb', 'rpm', 'dmg', 'pkg', 'jar', 'apk', 'bin', 'img', 'cab',
  'mp3', 'aac', 'flac', 'ogg', 'opus', 'm4a', 'wav',
  'mp4', 'mkv', 'avi', 'mov', 'webm', '3gp', 'mpeg', 'srt', 'pdf'
]);

export const DEFAULTS = Object.freeze({
  autoCapture: true,
  includeTypes: DEFAULT_INCLUDE_TYPES,
  excludeTypes: [],
  blockedSites: [],
  minSizeKb: 0,
  bypassKey: 'Delete',
  pageButtons: true,
  contextMenu: true,
  silentAdd: false,
  silentStart: false,
  handoff: true,
  closeTab: true,
  sendHeaders: true,
  manualPort: null,
  manualKey: ''
});

export const PORT_MIN = 1024;
export const PORT_MAX = 65535;

export function parseTypes(input) {
  const parts = Array.isArray(input) ? input : String(input ?? '').split(/[\s,;]+/);
  const out = [];
  for (const part of parts) {
    if (typeof part !== 'string') continue;
    const ext = part.trim().toLowerCase().replace(/^[*.]+/, '');
    if (!/^[a-z0-9_+-]{1,16}$/.test(ext)) continue;
    if (NEVER_TYPES.includes(ext)) continue;
    if (!out.includes(ext)) out.push(ext);
  }
  return out;
}

export function parseLines(input) {
  const parts = Array.isArray(input) ? input : String(input ?? '').split(/\r?\n/);
  const out = [];
  for (const part of parts) {
    if (typeof part !== 'string') continue;
    const line = part.trim();
    if (line && line.length <= 500 && !out.includes(line)) out.push(line);
  }
  return out;
}

export function parsePort(value) {
  if (value === null || value === undefined || value === '') return null;
  const n = Number(value);
  if (!Number.isInteger(n) || n < PORT_MIN || n > PORT_MAX) return undefined;
  return n;
}

function bool(value, fallback) {
  return typeof value === 'boolean' ? value : fallback;
}

export function validate(raw) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const minSize = Number(src.minSizeKb);
  const port = parsePort(src.manualPort);
  const key = typeof src.bypassKey === 'string' && src.bypassKey.length > 0 && src.bypassKey.length <= 24
    ? src.bypassKey
    : DEFAULTS.bypassKey;
  return {
    autoCapture: bool(src.autoCapture, DEFAULTS.autoCapture),
    includeTypes: Array.isArray(src.includeTypes) ? parseTypes(src.includeTypes) : [...DEFAULTS.includeTypes],
    excludeTypes: Array.isArray(src.excludeTypes) ? parseTypes(src.excludeTypes) : [],
    blockedSites: Array.isArray(src.blockedSites) ? parseLines(src.blockedSites) : [],
    minSizeKb: Number.isInteger(minSize) && minSize >= 0 && minSize <= 1e9 ? minSize : DEFAULTS.minSizeKb,
    bypassKey: key,
    pageButtons: bool(src.pageButtons, DEFAULTS.pageButtons),
    contextMenu: bool(src.contextMenu, DEFAULTS.contextMenu),
    silentAdd: bool(src.silentAdd, DEFAULTS.silentAdd),
    silentStart: bool(src.silentStart, DEFAULTS.silentStart),
    handoff: bool(src.handoff, DEFAULTS.handoff),
    closeTab: bool(src.closeTab, DEFAULTS.closeTab),
    sendHeaders: bool(src.sendHeaders, DEFAULTS.sendHeaders),
    manualPort: port === undefined ? null : port,
    manualKey: typeof src.manualKey === 'string' ? src.manualKey.trim().slice(0, 200) : ''
  };
}

export function effectiveSilentStart(options) {
  return Boolean(options.silentAdd && options.silentStart);
}

export async function load() {
  const stored = await api.storage.local.get(STORAGE_KEY);
  return validate(stored?.[STORAGE_KEY]);
}

export async function save(patch) {
  const next = validate({ ...(await load()), ...patch });
  await api.storage.local.set({ [STORAGE_KEY]: next });
  return next;
}

export async function reset(keys) {
  const next = { ...(await load()) };
  for (const key of keys) next[key] = Array.isArray(DEFAULTS[key]) ? [...DEFAULTS[key]] : DEFAULTS[key];
  return save(next);
}

export function onChange(callback) {
  api.storage.onChanged.addListener((changes, area) => {
    if (area === 'local' && changes[STORAGE_KEY]) callback(validate(changes[STORAGE_KEY].newValue));
  });
}
