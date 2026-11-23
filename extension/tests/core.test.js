import test from 'node:test';
import assert from 'node:assert/strict';

import {
  decide, parseDisposition, parseContentType, extensionOf, fileNameFromUrl,
  matchesPattern, isBlocked, classifyMime, decodeRfc2047, handoffScheme
} from '../src/capture.js';
import { createDeduper, keysFor, normalizeUrl } from '../src/dedupe.js';
import { buildHeaders, filterHeaders, cookieHeader, responseHeader } from '../src/headers.js';
import {
  validate, parseTypes, parseLines, parsePort, DEFAULTS, NEVER_TYPES, effectiveSilentStart
} from '../src/options-store.js';
import { parseM3u8, mimeToExtension, isYoutube, mediaName } from '../src/media.js';

const opts = (patch = {}) => ({ ...validate({}), ...patch });
const base = (patch = {}) => ({
  url: 'https://files.example.com/get/42',
  chain: ['https://files.example.com/get/42'],
  method: 'GET',
  status: 200,
  contentType: 'application/octet-stream',
  disposition: '',
  contentLength: 5_000_000,
  ...patch
});

test('attachment is captured whatever the type', () => {
  const r = decide(base({ contentType: 'text/plain', disposition: 'attachment; filename="log.txt"' }), opts());
  assert.equal(r.capture, true);
  assert.equal(r.reason, 'attachment');
  assert.equal(r.suggestedName, 'log.txt');
});

test('non-page MIME is captured without an extension match', () => {
  assert.equal(decide(base({ contentType: 'application/zip' }), opts()).capture, true);
  assert.equal(decide(base({ contentType: 'application/octet-stream' }), opts()).reason, 'mime-generic');
  assert.equal(decide(base({ contentType: 'video/mp4' }), opts()).capture, true);
});

test('extensionless names follow the same rules', () => {
  const r = decide(base({ url: 'https://x.example/download', chain: [], contentType: 'application/x-tar' }), opts());
  assert.equal(r.capture, true);
});

test('pages and inline types stay in the browser', () => {
  for (const type of ['text/html; charset=utf-8', 'application/xhtml+xml']) {
    assert.equal(decide(base({ contentType: type }), opts()).reason, 'page');
  }
  assert.equal(decide(base({ contentType: 'text/html', disposition: 'attachment; filename=a.html' }), opts()).capture, false);
  assert.equal(decide(base({ contentType: 'text/plain' }), opts()).capture, false);
  assert.equal(decide(base({ contentType: 'application/json' }), opts()).capture, false);
  assert.equal(decide(base({ contentType: 'image/png' }), opts()).capture, false);
  assert.equal(decide(base({ contentType: 'image/png', disposition: 'attachment' }), opts()).capture, true);
});

test('include list overrides inline text, exclude list overrides everything but switches', () => {
  const srt = base({ url: 'https://x.example/sub.srt', chain: [], contentType: 'text/plain' });
  assert.equal(decide(srt, opts()).reason, 'include-type');
  assert.equal(decide(srt, opts({ includeTypes: [] })).capture, false);
  const zip = base({ url: 'https://x.example/a.zip', chain: [], contentType: 'application/zip', disposition: 'attachment' });
  assert.equal(decide(zip, opts({ excludeTypes: ['zip'] })).reason, 'excluded-type');
});

test('never-capture extensions are not captured on a generic response', () => {
  const php = base({ url: 'https://x.example/index.php', chain: [], contentType: 'application/octet-stream' });
  assert.equal(decide(php, opts()).reason, 'never-type');
  const sent = base({ url: 'https://x.example/export.php', chain: [], contentType: 'application/zip' });
  assert.equal(decide(sent, opts()).capture, true);
});

test('POST, status, scheme, switch, bypass and size rules', () => {
  assert.equal(decide(base({ method: 'POST' }), opts()).reason, 'post');
  assert.equal(decide(base({ status: 404 }), opts()).reason, 'status');
  assert.equal(decide(base({ url: 'blob:https://x/1' }), opts()).reason, 'scheme');
  assert.equal(decide(base(), opts({ autoCapture: false })).reason, 'off');
  assert.equal(decide(base(), opts(), true).reason, 'bypass');
  assert.equal(decide(base({ contentLength: 10_000 }), opts({ minSizeKb: 100 })).reason, 'too-small');
  assert.equal(decide(base({ contentLength: -1 }), opts({ minSizeKb: 100 })).capture, true);
});

test('blocked sites match the url, chain and page', () => {
  const o = opts({ blockedSites: ['example.org', 'https://cdn.example.com/private/*'] });
  assert.equal(decide(base({ pageUrl: 'https://www.example.org/x' }), o).reason, 'blocked-site');
  assert.equal(decide(base({ chain: ['https://cdn.example.com/private/a.zip'] }), o).reason, 'blocked-site');
  assert.equal(decide(base(), o).capture, true);
  assert.equal(matchesPattern('https://a.example.org/x', 'example.org'), true);
  assert.equal(matchesPattern('https://notexample.org/x', 'example.org'), false);
  assert.equal(matchesPattern('https://a.test/x/y.zip', '*.test/x/*'), true);
  assert.equal(isBlocked(['https://a.test/'], []), false);
});

test('browser-started download with no type is captured, a page is not', () => {
  assert.equal(decide(base({ contentType: '', browserDownload: true }), opts()).reason, 'browser-download');
  assert.equal(decide(base({ contentType: '' }), opts()).capture, false);
});

test('content disposition parsing', () => {
  assert.deepEqual(parseDisposition('attachment; filename="a b.zip"'), { type: 'attachment', filename: 'a b.zip' });
  assert.equal(parseDisposition("attachment; filename*=UTF-8''%E2%82%AC%20rates.pdf").filename, '€ rates.pdf');
  assert.equal(parseDisposition('attachment; filename="x.zip"; filename*=UTF-8\'\'y.zip').filename, 'y.zip');
  assert.equal(parseDisposition('attachment; filename==?UTF-8?B?w6l0w6kuemlw?=').filename, 'été.zip');
  assert.equal(parseDisposition('inline').type, 'inline');
  assert.equal(parseDisposition('attachment; filename="../../evil.exe"').filename, '.._.._evil.exe');
  assert.equal(decodeRfc2047('=?utf-8?Q?caf=C3=A9?='), 'café');
});

test('small helpers', () => {
  assert.equal(parseContentType('Text/HTML; charset=utf-8'), 'text/html');
  assert.equal(extensionOf('a.tar.GZ'), 'gz');
  assert.equal(extensionOf('.bashrc'), '');
  assert.equal(extensionOf('noext'), '');
  assert.equal(fileNameFromUrl('https://x/a/b%20c.zip?x=1'), 'b c.zip');
  assert.equal(classifyMime('application/pdf'), 'download');
  assert.equal(handoffScheme('blob:https://x/1'), 'blob');
  assert.equal(handoffScheme('https://x'), '');
});

test('dedupe claims the whole chain once inside the window', () => {
  let t = 1000;
  const d = createDeduper({ now: () => t });
  const keys = keysFor({ chain: ['https://a/x#frag', 'https://b/y'], downloadId: 7 });
  assert.deepEqual(keys.sort(), ['d:7', 'u:https://a/x', 'u:https://b/y']);
  assert.equal(d.claim(keys).ok, true);
  assert.equal(d.claim(keysFor({ chain: ['https://b/y'] })).ok, false);
  d.settle(keys, 'accepted');
  assert.equal(d.claim(keysFor({ downloadId: 7 })).status, 'accepted');
  t += 15001;
  assert.equal(d.claim(keys).ok, true);
});

test('dedupe failure state is short and release frees keys', () => {
  let t = 0;
  const d = createDeduper({ now: () => t });
  const keys = keysFor({ chain: ['https://a/x'] });
  d.claim(keys);
  d.settle(keys, 'failed');
  assert.equal(d.claim(keys).status, 'failed');
  t += 4001;
  assert.equal(d.claim(keys).ok, true);
  d.release(keys);
  assert.equal(d.status(keys), null);
  assert.equal(normalizeUrl('https://a/#x'), 'https://a/');
});

test('headers use the allow-list and honour the switch', () => {
  const wire = [
    { name: 'cookie', value: 'a=b' }, { name: 'Range', value: 'bytes=0-' }, { name: 'Sec-Fetch-Mode', value: 'navigate' },
    { name: 'Host', value: 'x' }, { name: 'If-None-Match', value: '"1"' }, { name: 'Referer', value: 'https://p/' },
    { name: 'Authorization', value: 'Bearer t' }, { name: 'Accept-Language', value: 'en' }, { name: 'Origin', value: 'https://p' }
  ];
  assert.deepEqual(Object.keys(filterHeaders(wire)).sort(), ['Accept-Language', 'Authorization', 'Cookie', 'Origin', 'Referer']);
  assert.equal(buildHeaders({ sendHeaders: false, wire }), null);
  const fb = buildHeaders({
    sendHeaders: true, wire: [], cookies: [{ name: 'a', value: '1', path: '/' }, { name: 'b', value: '2', path: '/x' }],
    referer: 'https://page/', userAgent: 'UA'
  });
  assert.deepEqual(fb, { Cookie: 'b=2; a=1', Referer: 'https://page/', 'User-Agent': 'UA' });
  assert.equal(cookieHeader([]), '');
  assert.equal(responseHeader([{ name: 'Content-Type', value: 'x/y' }], 'content-type'), 'x/y');
});

test('options validation falls back per key and clamps', () => {
  const v = validate({ autoCapture: 'yes', minSizeKb: -5, manualPort: 80, bypassKey: '', includeTypes: ['ZIP', '.rar', 'html', 'php', 'bad ext', 'zip'] });
  assert.equal(v.autoCapture, DEFAULTS.autoCapture);
  assert.equal(v.minSizeKb, 0);
  assert.equal(v.manualPort, null);
  assert.equal(v.bypassKey, 'Delete');
  assert.deepEqual(v.includeTypes, ['zip', 'rar']);
  assert.deepEqual(validate(null).includeTypes, [...DEFAULTS.includeTypes]);
  assert.equal(validate({ manualPort: '15161' }).manualPort, 15161);
  assert.equal(validate({ manualKey: '  abc  ' }).manualKey, 'abc');
});

test('type lists strip the never-capture types', () => {
  assert.deepEqual(parseTypes('zip, HTML  php\nxhtml .iso *.7z asp aspx jsp htm'), ['zip', 'iso', '7z']);
  for (const t of NEVER_TYPES) assert.equal(DEFAULTS.includeTypes.includes(t), false);
  assert.deepEqual(parseLines(' a \n\n b\na'), ['a', 'b']);
  assert.equal(parsePort('70000'), undefined);
  assert.equal(parsePort(''), null);
  assert.equal(effectiveSilentStart({ silentAdd: false, silentStart: true }), false);
  assert.equal(effectiveSilentStart({ silentAdd: true, silentStart: true }), true);
});

test('hls parsing: master, media, encrypted', () => {
  const master = '#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,FRAME-RATE=29.97\nlow/index.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2000000,RESOLUTION=1280x720\nhttps://cdn.example/hi.m3u8\n';
  const m = parseM3u8(master, 'https://cdn.example/v/master.m3u8');
  assert.equal(m.kind, 'master');
  assert.equal(m.variants[0].url, 'https://cdn.example/v/low/index.m3u8');
  assert.equal(m.variants[0].resolution, '640x360');
  assert.equal(m.variants[1].bandwidth, 2000000);
  const media = '#EXTM3U\n#EXTINF:4.0,\na.ts\n#EXTINF:6.5,\nb.ts\n#EXT-X-ENDLIST\n';
  const p = parseM3u8(media, 'https://cdn.example/v/index.m3u8');
  assert.equal(p.kind, 'media');
  assert.equal(p.duration, 10.5);
  assert.equal(p.encrypted, false);
  assert.equal(parseM3u8('#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI="k"\n#EXTINF:4,\na.ts\n', 'https://c/i.m3u8').encrypted, true);
  assert.equal(parseM3u8('#EXTM3U\n#EXT-X-MAP:URI="init.mp4"\n#EXTINF:4,\na.m4s\n', 'https://c/i.m3u8').supported, false);
});

test('media naming', () => {
  assert.equal(mimeToExtension('video/mp4'), 'mp4');
  assert.equal(isYoutube('https://www.youtube.com/watch?v=1'), true);
  assert.equal(isYoutube('https://example.com/'), false);
  assert.equal(mediaName({ disposition: 'attachment; filename="clip.mkv"', url: 'https://x/a', mime: 'video/x-matroska' }), 'clip.mkv');
  assert.equal(mediaName({ url: 'https://x/path/movie', mime: 'video/mp4', title: 'T' }), 'movie.mp4');
});
