import { NEVER_TYPES } from './options-store.js';

const GENERIC_MIMES = new Set([
  'application/octet-stream', 'binary/octet-stream', 'application/force-download',
  'application/x-download', 'application/download', 'application/unknown'
]);

const INLINE_APPLICATION = /^application\/(?:json|xml|javascript|x-javascript|ecmascript|x-ndjson|ld\+json|manifest\+json|[a-z0-9.+-]+\+(?:json|xml))$/;

export function parseContentType(value) {
  if (typeof value !== 'string') return '';
  return value.split(';')[0].trim().toLowerCase();
}

function bytesFromQuoted(text) {
  const bytes = [];
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === '_') bytes.push(0x20);
    else if (ch === '=' && /^[0-9a-f]{2}$/i.test(text.slice(i + 1, i + 3))) {
      bytes.push(parseInt(text.slice(i + 1, i + 3), 16));
      i += 2;
    } else bytes.push(ch.charCodeAt(0) & 0xff);
  }
  return Uint8Array.from(bytes);
}

function percentBytes(text) {
  const bytes = [];
  for (let i = 0; i < text.length; i += 1) {
    if (text[i] === '%' && /^[0-9a-f]{2}$/i.test(text.slice(i + 1, i + 3))) {
      bytes.push(parseInt(text.slice(i + 1, i + 3), 16));
      i += 2;
    } else bytes.push(text.charCodeAt(i) & 0xff);
  }
  return Uint8Array.from(bytes);
}

function decodeBytes(bytes, charset) {
  try {
    return new TextDecoder(charset || 'utf-8').decode(bytes);
  } catch {
    return new TextDecoder('utf-8').decode(bytes);
  }
}

export function decodeRfc2047(text) {
  return String(text).replace(/=\?([^?]+)\?([bBqQ])\?([^?]*)\?=/g, (whole, charset, enc, body) => {
    try {
      if (enc.toLowerCase() === 'b') {
        const raw = atob(body);
        return decodeBytes(Uint8Array.from(raw, (c) => c.charCodeAt(0)), charset);
      }
      return decodeBytes(bytesFromQuoted(body), charset);
    } catch {
      return whole;
    }
  });
}

function cleanName(name) {
  return name.replace(/[\\/]+/g, '_').replace(/[\u0000-\u001f]/g, '').trim();
}

export function parseDisposition(value) {
  const out = { type: '', filename: '' };
  if (typeof value !== 'string' || !value) return out;
  const type = /^\s*([a-z-]+)/i.exec(value);
  if (type) out.type = type[1].toLowerCase();

  const star = /filename\*\s*=\s*([^;]+)/i.exec(value);
  if (star) {
    const parts = /^\s*([^']*)'[^']*'(.*?)\s*$/.exec(star[1]);
    if (parts) {
      const encoded = parts[2].replace(/"/g, '');
      const charset = parts[1].toLowerCase();
      try {
        out.filename = cleanName(!charset || charset === 'utf-8'
          ? decodeURIComponent(encoded)
          : decodeBytes(percentBytes(encoded), charset));
      } catch {
        out.filename = cleanName(encoded);
      }
    }
  }
  if (!out.filename) {
    const plain = /filename\s*=\s*(?:"((?:[^"\\]|\\.)*)"|([^;]+))/i.exec(value);
    if (plain) {
      const raw = (plain[1] !== undefined ? plain[1].replace(/\\(.)/g, '$1') : plain[2].trim());
      out.filename = cleanName(decodeRfc2047(raw));
    }
  }
  return out;
}

export function fileNameFromUrl(url) {
  try {
    const path = new URL(url).pathname;
    const last = path.split('/').filter(Boolean).pop() || '';
    try {
      return decodeURIComponent(last);
    } catch {
      return last;
    }
  } catch {
    return '';
  }
}

export function extensionOf(name) {
  if (typeof name !== 'string') return '';
  const i = name.lastIndexOf('.');
  if (i <= 0 || i === name.length - 1) return '';
  const ext = name.slice(i + 1).toLowerCase();
  return /^[a-z0-9_+-]{1,16}$/.test(ext) ? ext : '';
}

export function schemeOf(url) {
  const m = /^([a-z][a-z0-9+.-]*):/i.exec(typeof url === 'string' ? url : '');
  return m ? m[1].toLowerCase() : '';
}

export function isHttpUrl(url) {
  const s = schemeOf(url);
  return s === 'http' || s === 'https';
}

export function handoffScheme(url) {
  const s = schemeOf(url);
  return s === 'blob' || s === 'data' ? s : '';
}

function escapeRegex(text) {
  return text.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
}

export function matchesPattern(url, pattern) {
  if (typeof url !== 'string' || typeof pattern !== 'string') return false;
  const p = pattern.trim();
  if (!p) return false;
  if (!p.includes('*') && !p.includes('://')) {
    let host = '';
    try {
      host = new URL(url).hostname.toLowerCase();
    } catch {
      return false;
    }
    const site = p.toLowerCase().replace(/\/.*$/, '');
    return host === site || host.endsWith(`.${site}`);
  }
  const body = p.split('*').map(escapeRegex).join('.*');
  const prefix = p.includes('://') ? '' : '(?:[a-z][a-z0-9+.-]*://)?';
  return new RegExp(`^${prefix}${body}$`, 'i').test(url);
}

export function isBlocked(urls, patterns) {
  if (!patterns || patterns.length === 0) return false;
  for (const url of urls) {
    if (!url) continue;
    for (const pattern of patterns) if (matchesPattern(url, pattern)) return true;
  }
  return false;
}

export function classifyMime(mime) {
  if (!mime) return 'unknown';
  if (mime === 'text/html' || mime === 'application/xhtml+xml') return 'page';
  if (GENERIC_MIMES.has(mime)) return 'generic';
  if (mime.startsWith('text/') || mime.startsWith('image/') || INLINE_APPLICATION.test(mime)) return 'inline';
  return 'download';
}

export function decide(ctx, options, bypass = false) {
  const no = (reason) => ({ capture: false, reason, name, suggestedName });
  const yes = (reason) => ({ capture: true, reason, name, suggestedName });

  const mime = parseContentType(ctx.contentType);
  const disposition = parseDisposition(ctx.disposition);
  const suggestedName = disposition.filename;
  const urls = [...(ctx.chain || []), ctx.url, ctx.pageUrl, ctx.originUrl];
  const name = suggestedName || fileNameFromUrl(ctx.url);
  const ext = extensionOf(name);
  const attachment = disposition.type === 'attachment';
  const kind = classifyMime(mime);

  if (!options.autoCapture) return no('off');
  if (!isHttpUrl(ctx.url)) return no('scheme');
  if (String(ctx.method || 'GET').toUpperCase() !== 'GET') return no('post');
  if (typeof ctx.status === 'number' && (ctx.status < 200 || ctx.status > 299)) return no('status');
  if (isBlocked(urls, options.blockedSites)) return no('blocked-site');
  if (bypass) return no('bypass');
  const size = Number(ctx.contentLength);
  if (options.minSizeKb > 0 && Number.isFinite(size) && size >= 0 && size < options.minSizeKb * 1024) return no('too-small');
  if (kind === 'page') return no('page');
  if (ext && options.excludeTypes.includes(ext)) return no('excluded-type');
  if (ext && NEVER_TYPES.includes(ext) && !attachment && kind !== 'download') return no('never-type');
  if (attachment) return yes('attachment');
  if (ext && options.includeTypes.includes(ext)) return yes('include-type');
  if (kind === 'download') return yes('mime');
  if (kind === 'generic') return yes('mime-generic');
  if (kind === 'unknown' && ctx.browserDownload) return yes('browser-download');
  return no(kind === 'inline' ? 'inline' : 'unknown-type');
}

export function buildItem({ link, headers, downloadPage, suggestedName, type = 'http' }) {
  return {
    type,
    link,
    headers: headers || null,
    downloadPage: downloadPage || null,
    suggestedName: suggestedName || null
  };
}
