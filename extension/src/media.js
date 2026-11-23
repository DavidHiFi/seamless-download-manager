import { parseDisposition, parseContentType, fileNameFromUrl, extensionOf } from './capture.js';

export const PLAYLIST_MAX_BYTES = 2 * 1024 * 1024;

const MIME_EXTENSIONS = {
  'video/mp4': 'mp4', 'video/webm': 'webm', 'video/x-matroska': 'mkv', 'video/quicktime': 'mov',
  'video/x-msvideo': 'avi', 'video/mpeg': 'mpeg', 'video/3gpp': '3gp', 'video/ogg': 'ogv',
  'video/mp2t': 'ts', 'video/x-flv': 'flv', 'video/x-ms-wmv': 'wmv',
  'audio/mpeg': 'mp3', 'audio/mp3': 'mp3', 'audio/aac': 'aac', 'audio/mp4': 'm4a', 'audio/x-m4a': 'm4a',
  'audio/ogg': 'ogg', 'audio/opus': 'opus', 'audio/webm': 'weba', 'audio/wav': 'wav', 'audio/x-wav': 'wav',
  'audio/flac': 'flac', 'audio/x-flac': 'flac'
};

export function mimeToExtension(mime) {
  return MIME_EXTENSIONS[parseContentType(mime)] || '';
}

export function isYoutube(url) {
  try {
    const host = new URL(url).hostname.toLowerCase();
    return host === 'youtu.be' || host === 'youtube.com' || host.endsWith('.youtube.com');
  } catch {
    return false;
  }
}

export function isPlaylistUrl(url) {
  try {
    return /\.m3u8$/i.test(new URL(url).pathname);
  } catch {
    return false;
  }
}

export function isMediaMime(mime) {
  const m = parseContentType(mime);
  return m.startsWith('video/') || m.startsWith('audio/');
}

export function shortHash(text) {
  let h = 5381;
  for (let i = 0; i < text.length; i += 1) h = ((h << 5) + h + text.charCodeAt(i)) >>> 0;
  return h.toString(36).slice(0, 6);
}

export function mediaName({ disposition, url, mime, title, forceExt }) {
  const fromHeader = parseDisposition(disposition).filename;
  let name = fromHeader || fileNameFromUrl(url);
  if (!name) name = `${(title || 'media').trim().slice(0, 60)}-${shortHash(url || '')}`;
  name = name.replace(/[\\/:*?"<>|]+/g, '_');
  if (!extensionOf(name)) {
    const ext = forceExt || mimeToExtension(mime);
    if (ext) name = `${name}.${ext}`;
  }
  return name;
}

export function hlsName(url, title) {
  const base = fileNameFromUrl(url).replace(/\.m3u8$/i, '').replace(/[\\/:*?"<>|]+/g, '_');
  const name = base && base.toLowerCase() !== 'index' && base.toLowerCase() !== 'playlist'
    ? base
    : `${(title || 'video').trim().slice(0, 60).replace(/[\\/:*?"<>|]+/g, '_')}-${shortHash(url || '')}`;
  return `${name}.ts`;
}

function attrs(line) {
  const out = {};
  const re = /([A-Z0-9-]+)=("([^"]*)"|[^,]*)/g;
  let m;
  while ((m = re.exec(line)) !== null) out[m[1]] = m[3] !== undefined ? m[3] : m[2];
  return out;
}

function resolve(ref, base) {
  try {
    return new URL(ref, base).href;
  } catch {
    return '';
  }
}

export function parseM3u8(text, baseUrl) {
  const lines = String(text).split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  if (lines.some((l) => l.startsWith('#EXT-X-STREAM-INF'))) {
    const variants = [];
    for (let i = 0; i < lines.length; i += 1) {
      if (!lines[i].startsWith('#EXT-X-STREAM-INF')) continue;
      const a = attrs(lines[i].slice(lines[i].indexOf(':') + 1));
      const uri = lines.slice(i + 1).find((l) => !l.startsWith('#'));
      const url = uri ? resolve(uri, baseUrl) : '';
      if (!url) continue;
      variants.push({
        url,
        bandwidth: Number(a.BANDWIDTH) || 0,
        resolution: a.RESOLUTION || '',
        frameRate: Number(a['FRAME-RATE']) || 0
      });
    }
    return { kind: 'master', variants };
  }

  let duration = 0;
  let encrypted = false;
  let mapped = false;
  const segments = [];
  for (const line of lines) {
    if (line.startsWith('#EXTINF:')) duration += parseFloat(line.slice(8)) || 0;
    else if (line.startsWith('#EXT-X-KEY') || line.startsWith('#EXT-X-SESSION-KEY')) {
      const method = (attrs(line.slice(line.indexOf(':') + 1)).METHOD || 'NONE').toUpperCase();
      if (method !== 'NONE') encrypted = true;
    } else if (line.startsWith('#EXT-X-MAP')) mapped = true;
    else if (!line.startsWith('#')) segments.push(line);
  }
  const allTs = segments.length > 0 && segments.every((s) => /\.ts$/i.test(s.split(/[?#]/)[0]));
  return {
    kind: 'media',
    encrypted,
    supported: !mapped && allTs,
    duration: Math.round(duration * 1000) / 1000,
    segments: segments.length
  };
}

async function readCapped(response, max) {
  const length = Number(response.headers.get('content-length'));
  if (Number.isFinite(length) && length > max) return null;
  if (!response.body?.getReader) {
    const text = await response.text();
    return text.length > max ? null : text;
  }
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let total = 0;
  let text = '';
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.byteLength;
    if (total > max) {
      await reader.cancel().catch(() => {});
      return null;
    }
    text += decoder.decode(value, { stream: true });
  }
  return text + decoder.decode();
}

export async function fetchPlaylist(url, fetchFn) {
  const response = await fetchFn(url, { credentials: 'include', cache: 'no-store' });
  if (!response.ok) return null;
  const type = parseContentType(response.headers.get('content-type') || '');
  if (type && !/mpegurl|text\/|octet-stream/.test(type)) return null;
  const text = await readCapped(response, PLAYLIST_MAX_BYTES);
  return text && text.trimStart().startsWith('#EXTM3U') ? text : null;
}

export async function detectHls(url, fetchFn, depth = 0) {
  if (depth > 2) return [];
  let text;
  try {
    text = await fetchPlaylist(url, fetchFn);
  } catch {
    return [];
  }
  if (!text) return [];
  const parsed = parseM3u8(text, url);
  if (parsed.kind === 'master') {
    const found = [];
    for (const variant of parsed.variants.slice(0, 12)) {
      const inner = await detectHls(variant.url, fetchFn, depth + 1);
      for (const item of inner) {
        found.push({
          ...item,
          bandwidth: item.bandwidth || variant.bandwidth,
          resolution: item.resolution || variant.resolution,
          frameRate: item.frameRate || variant.frameRate
        });
      }
    }
    return found;
  }
  if (parsed.encrypted || !parsed.supported) return [];
  return [{ type: 'hls', url, duration: parsed.duration, bandwidth: 0, resolution: '', frameRate: 0 }];
}

export function describeItem(item) {
  const parts = [];
  if (item.duration) {
    const s = Math.round(item.duration);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = String(s % 60).padStart(2, '0');
    parts.push(h ? `${h}:${String(m).padStart(2, '0')}:${sec}` : `${m}:${sec}`);
  }
  if (item.size > 0) parts.push(formatSize(item.size));
  if (item.resolution) parts.push(item.frameRate ? `${item.resolution} ${Math.round(item.frameRate)}fps` : item.resolution);
  if (item.bandwidth) parts.push(`${Math.round(item.bandwidth / 1000)} kbps`);
  const ext = extensionOf(item.name);
  if (ext) parts.push(ext);
  return parts.join('  ');
}

export function formatSize(bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i += 1;
  }
  return `${value >= 100 || i === 0 ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

export function addMedia(list, item, limit = 40) {
  if (list.some((existing) => existing.url === item.url)) return false;
  list.push(item);
  if (list.length > limit) list.shift();
  return true;
}
