import { api, action, isFirefox } from './platform.js';
import * as store from './options-store.js';
import { decide, buildItem, isHttpUrl, handoffScheme, isBlocked } from './capture.js';
import { createDeduper, keysFor } from './dedupe.js';
import { buildHeaders, filterHeaders, responseHeader } from './headers.js';
import * as protocol from './protocol.js';
import {
  detectHls, hlsName, isPlaylistUrl, isMediaMime, isYoutube, mediaName, shortHash, addMedia
} from './media.js';
import { createPersisted } from './state.js';

const URLS = ['http://*/*', 'https://*/*'];
const PENDING_MAX_AGE = 90000;
const PENDING_MAX_COUNT = 300;
const WATCH_MAX_AGE = 6 * 3600 * 1000;
const NEW_TAB_AGE = 60000;
const BYPASS_MAX_AGE = 15000;
const LINK_LIMIT = 500;

const pendingState = createPersisted('pending');
const dedupeState = createPersisted('dedupe');
const watchState = createPersisted('watch');
const tabsState = createPersisted('tabs');
const mediaState = createPersisted('media');
const bypassState = createPersisted('bypass');

const pending = pendingState.data;
const watch = watchState.data;
const newTabs = tabsState.data;
const media = mediaState.data;
const bypass = bypassState.data;
const deduper = createDeduper({ store: dedupeState.data });
const inflight = new Map();
const storeIds = new Map();

const ready = Promise.all([
  pendingState.ready, dedupeState.ready, watchState.ready, tabsState.ready, mediaState.ready, bypassState.ready
]).then(() => syncKeepAlive());

let optionsCache = null;
let lastPrune = 0;
let keepPort = null;
let badgeTimer = null;

async function getOptions() {
  if (!optionsCache) optionsCache = await store.load();
  return optionsCache;
}

function isBypass() {
  return Boolean(bypass.since) && Date.now() - bypass.since < BYPASS_MAX_AGE;
}

function prune() {
  const now = Date.now();
  lastPrune = now;
  for (const id of Object.keys(pending)) if (now - pending[id].t > PENDING_MAX_AGE) delete pending[id];
  const ids = Object.keys(pending);
  if (ids.length > PENDING_MAX_COUNT) {
    ids.sort((a, b) => pending[a].t - pending[b].t);
    for (const id of ids.slice(0, ids.length - PENDING_MAX_COUNT)) delete pending[id];
  }
  for (const id of Object.keys(newTabs)) if (now - newTabs[id].t > NEW_TAB_AGE * 5) delete newTabs[id];
  for (const id of Object.keys(watch)) if (now - watch[id].t > WATCH_MAX_AGE) delete watch[id];
}

function badge(text, color) {
  if (!action?.setBadgeText) return;
  try {
    action.setBadgeText({ text });
    if (color && action.setBadgeBackgroundColor) action.setBadgeBackgroundColor({ color });
  } catch {}
}

async function notify(tabId, text, kind = 'error') {
  const message = { type: 'toast', text, kind };
  const candidates = [];
  if (tabId >= 0) candidates.push(tabId);
  try {
    const [active] = await api.tabs.query({ active: true, currentWindow: true });
    if (active?.id !== undefined && active.id !== tabId) candidates.push(active.id);
  } catch {}
  for (const id of candidates) {
    try {
      await api.tabs.sendMessage(id, message);
      return;
    } catch {}
  }
  if (kind === 'error') {
    badge('!', '#F38BA8');
    clearTimeout(badgeTimer);
    badgeTimer = setTimeout(() => badge(''), 8000);
  }
}

function syncKeepAlive() {
  if (isFirefox) return;
  const need = Object.keys(watch).length > 0;
  if (need && !keepPort) {
    try {
      keepPort = api.runtime.connectNative(protocol.HOST);
      keepPort.onDisconnect.addListener(() => {
        keepPort = null;
        void api.runtime.lastError;
      });
    } catch {
      keepPort = null;
    }
  } else if (!need && keepPort) {
    try {
      keepPort.disconnect();
    } catch {}
    keepPort = null;
  }
}

function recordRequest(details) {
  if (!isHttpUrl(details.url)) return;
  const now = Date.now();
  if (now - lastPrune > 10000) prune();
  let entry = pending[details.requestId];
  if (!entry) {
    entry = pending[details.requestId] = {
      id: details.requestId,
      chain: [],
      method: details.method,
      tabId: details.tabId,
      type: details.type,
      origin: details.originUrl || details.documentUrl || '',
      t: now
    };
  }
  if (entry.chain[entry.chain.length - 1] !== details.url) entry.chain.push(details.url);
  entry.method = details.method;
  if (optionsCache?.sendHeaders !== false) entry.headers = filterHeaders(details.requestHeaders);
  pendingState.touch();
}

function recordResponse(details) {
  const entry = pending[details.requestId];
  if (!entry) return null;
  const length = Number(responseHeader(details.responseHeaders, 'content-length'));
  entry.resp = {
    status: details.statusCode,
    contentType: responseHeader(details.responseHeaders, 'content-type'),
    disposition: responseHeader(details.responseHeaders, 'content-disposition'),
    contentLength: Number.isFinite(length) && length >= 0 && responseHeader(details.responseHeaders, 'content-length') !== '' ? length : -1
  };
  pendingState.touch();
  return entry;
}

function findPending(...urls) {
  const wanted = urls.filter(Boolean);
  let best = null;
  for (const entry of Object.values(pending)) {
    if (!entry.chain.some((u) => wanted.includes(u))) continue;
    if (!best || entry.t > best.t) best = entry;
  }
  return best;
}

async function storeIdFor(tabId) {
  if (!(tabId >= 0) || !api.cookies?.getAllCookieStores) return undefined;
  if (storeIds.has(tabId)) return storeIds.get(tabId);
  try {
    const stores = await api.cookies.getAllCookieStores();
    const found = stores.find((s) => s.tabIds?.includes(tabId));
    if (found) storeIds.set(tabId, found.id);
    return found?.id;
  } catch {
    return undefined;
  }
}

async function collectHeaders({ options, wire, url, tabId, referer }) {
  if (!options.sendHeaders) return null;
  let cookies = [];
  if (!wire?.Cookie && api.cookies?.getAll) {
    try {
      const query = { url };
      const storeId = await storeIdFor(tabId);
      if (storeId) query.storeId = storeId;
      if (isFirefox) query.firstPartyDomain = null;
      cookies = await api.cookies.getAll(query);
    } catch {}
  }
  return buildHeaders({
    sendHeaders: true,
    wire,
    cookies,
    referer,
    userAgent: globalThis.navigator?.userAgent
  });
}

async function tabUrl(tabId) {
  if (!(tabId >= 0)) return '';
  try {
    return (await api.tabs.get(tabId)).url || '';
  } catch {
    return '';
  }
}

async function closeEmptyTab(tabId, chain) {
  if (!(tabId >= 0)) return;
  const born = newTabs[tabId];
  if (!born || Date.now() - born.t > NEW_TAB_AGE) return;
  try {
    const tab = await api.tabs.get(tabId);
    const urls = [tab.url, tab.pendingUrl].filter(Boolean);
    const blank = urls.every((u) => u === 'about:blank' || u === 'about:newtab' || chain.includes(u));
    if (!blank) return;
    const siblings = await api.tabs.query({ windowId: tab.windowId });
    if (siblings.length < 2) return;
    await api.tabs.remove(tabId);
  } catch {}
}

async function cancelBrowserDownload(id) {
  try {
    await api.downloads.cancel(id);
  } catch {}
  try {
    await api.downloads.erase({ id });
  } catch {}
}

async function resolveSeen(keys, seen) {
  if (seen === 'accepted') return { accepted: true, dup: true };
  if (seen === 'sending') {
    const promise = keys.map((k) => inflight.get(k)).find(Boolean);
    if (promise) return { ...(await promise), dup: true };
  }
  return { accepted: false, dup: true };
}

async function sendCapture({ ctx, verdict, entry, tabId, options }) {
  const page = entry?.origin || ctx.pageUrl || (await tabUrl(tabId));
  const headers = await collectHeaders({
    options,
    wire: entry?.headers,
    url: ctx.url,
    tabId,
    referer: page && page !== ctx.url ? page : ''
  });
  const item = buildItem({
    link: ctx.url,
    headers,
    downloadPage: page && page !== ctx.url ? page : null,
    suggestedName: verdict.suggestedName
  });
  const reply = await protocol.add([item], { silentAdd: options.silentAdd, silentStart: options.silentStart });
  if (reply.ok) {
    if (options.closeTab) await closeEmptyTab(tabId, ctx.chain);
    if (options.silentAdd) notify(tabId, 'Sent to Seamless.', 'ok');
    return { accepted: true };
  }
  notify(tabId, `The browser is keeping this download. ${reply.error}`, 'error');
  return { accepted: false, error: reply.error };
}

async function captureFlow({ ctx, verdict, entry, keys, tabId, options }) {
  const seen = deduper.status(keys);
  if (seen) return resolveSeen(keys, seen);
  deduper.claim(keys);
  dedupeState.touch();
  const promise = sendCapture({ ctx, verdict, entry, tabId, options }).catch((error) => ({
    accepted: false,
    error: String(error?.message || error)
  }));
  for (const key of keys) inflight.set(key, promise);
  const result = await promise;
  for (const key of keys) inflight.delete(key);
  deduper.settle(keys, result.accepted ? 'accepted' : 'failed');
  dedupeState.touch();
  return result;
}

function contextFromEntry(entry, url) {
  const resp = entry.resp || {};
  return {
    url,
    chain: entry.chain,
    method: entry.method,
    status: resp.status,
    contentType: resp.contentType || '',
    disposition: resp.disposition || '',
    contentLength: resp.contentLength ?? -1,
    pageUrl: entry.origin || '',
    originUrl: entry.origin || ''
  };
}

function httpOnly(url) {
  return isHttpUrl(url) ? url : null;
}

function watchForHandoff(item, options) {
  if (!options.handoff) return;
  watch[item.id] = { url: httpOnly(item.finalUrl || item.url), referrer: httpOnly(item.referrer), t: Date.now() };
  watchState.touch();
  syncKeepAlive();
  if (item.state === 'complete') finishHandoff(item.id);
}

async function finishHandoff(id) {
  const entry = watch[id];
  if (!entry) return;
  delete watch[id];
  watchState.touch();
  syncKeepAlive();
  const options = await getOptions();
  if (!options.handoff) return;
  let item;
  try {
    [item] = await api.downloads.search({ id });
  } catch {
    return;
  }
  if (!item?.filename || item.exists === false) return;
  const reply = await protocol.handoff({ path: item.filename, url: entry.url, referrer: entry.referrer });
  if (!reply.ok) notify(-1, `A finished download could not be handed to Seamless. ${reply.error}`, 'error');
}

async function handleCreated(item) {
  if (item.byExtensionId) return;
  await ready;
  const options = await getOptions();
  const url = item.finalUrl || item.url;
  const entry = findPending(item.url, item.finalUrl);

  if (handoffScheme(item.url) || handoffScheme(url) || (entry && entry.method !== 'GET')) {
    watchForHandoff(item, options);
    return;
  }
  if (!isHttpUrl(url) || (item.state && item.state !== 'in_progress')) return;

  const chain = entry?.chain?.length ? entry.chain : [item.url, item.finalUrl].filter(Boolean);
  const resp = entry?.resp;
  const size = item.totalBytes > 0 ? item.totalBytes : item.fileSize > 0 ? item.fileSize : -1;
  const ctx = {
    url,
    chain,
    method: entry?.method || 'GET',
    status: resp?.status ?? 200,
    contentType: resp?.contentType || item.mime || '',
    disposition: resp?.disposition || '',
    contentLength: resp?.contentLength ?? size,
    pageUrl: entry?.origin || item.referrer || '',
    originUrl: item.referrer || '',
    browserDownload: true
  };
  const keys = keysFor({ chain, requestId: entry?.id, downloadId: item.id });

  const seen = deduper.status(keys);
  if (seen) {
    const result = await resolveSeen(keys, seen);
    if (result.accepted) await cancelBrowserDownload(item.id);
    return;
  }

  const verdict = decide(ctx, options, isBypass());
  if (!verdict.capture) return;

  let paused = false;
  try {
    await api.downloads.pause(item.id);
    paused = true;
  } catch {}

  const result = await captureFlow({ ctx, verdict, entry, keys, tabId: entry?.tabId ?? -1, options });
  if (result.accepted) {
    await cancelBrowserDownload(item.id);
  } else if (paused) {
    try {
      await api.downloads.resume(item.id);
    } catch {}
  }
}

async function onHeadersReceivedBlocking(details) {
  const entry = recordResponse(details);
  if (!entry || details.method !== 'GET') return {};
  await ready;
  const options = await getOptions();
  if (!options.autoCapture) return {};
  const ctx = contextFromEntry(entry, details.url);
  const verdict = decide(ctx, options, isBypass());
  if (!verdict.capture) return {};
  const keys = keysFor({ chain: entry.chain, requestId: entry.id });
  const result = await captureFlow({ ctx, verdict, entry, keys, tabId: details.tabId, options });
  return result.accepted ? { cancel: true } : {};
}

function parseTotal(range) {
  const m = /\/(\d+)\s*$/.exec(range || '');
  return m ? Number(m[1]) : -1;
}

async function handleMedia(details) {
  if (!(details.tabId >= 0)) return;
  await ready;
  const options = await getOptions();
  if (!options.pageButtons) return;
  const hls = details.type === 'xmlhttprequest';
  if (hls && !isPlaylistUrl(details.url)) return;
  const mime = responseHeader(details.responseHeaders, 'content-type');
  if (!hls && !isMediaMime(mime)) return;

  let tab;
  try {
    tab = await api.tabs.get(details.tabId);
  } catch {
    return;
  }
  const page = tab.url || '';
  if (!page || isYoutube(page) || isBlocked([page, details.url], options.blockedSites)) return;

  const found = [];
  if (hls) {
    for (const stream of await detectHls(details.url, (u, o) => fetch(u, o))) {
      found.push({
        id: shortHash(stream.url),
        type: 'hls',
        url: stream.url,
        name: hlsName(stream.url, tab.title),
        mime: 'application/vnd.apple.mpegurl',
        size: -1,
        duration: stream.duration,
        bandwidth: stream.bandwidth,
        resolution: stream.resolution,
        frameRate: stream.frameRate
      });
    }
  } else {
    const range = parseTotal(responseHeader(details.responseHeaders, 'content-range'));
    const length = Number(responseHeader(details.responseHeaders, 'content-length') || -1);
    const size = range >= 0 ? range : details.statusCode === 206 ? -1 : length;
    if (options.minSizeKb > 0 && size >= 0 && size < options.minSizeKb * 1024) return;
    found.push({
      id: shortHash(details.url),
      type: 'http',
      url: details.url,
      name: mediaName({ disposition: responseHeader(details.responseHeaders, 'content-disposition'), url: details.url, mime, title: tab.title }),
      mime,
      size
    });
  }
  if (found.length === 0) return;

  const list = (media[details.tabId] ??= []);
  let changed = false;
  for (const item of found) changed = addMedia(list, item) || changed;
  if (!changed) return;
  mediaState.touch();
  pushMedia(details.tabId);
}

function pushMedia(tabId) {
  api.tabs.sendMessage(tabId, { type: 'media', items: media[tabId] || [] }).catch(() => {});
}

async function sendItems(items, tabId, options) {
  const reply = await protocol.add(items, { silentAdd: options.silentAdd, silentStart: options.silentStart });
  if (reply.ok) {
    if (options.silentAdd) notify(tabId, items.length > 1 ? `Sent ${items.length} links to Seamless.` : 'Sent to Seamless.', 'ok');
  } else {
    notify(tabId, reply.error, 'error');
  }
  return reply;
}

async function sendLinks(links, tab, options, pageUrl) {
  const page = pageUrl || tab?.url || '';
  const tabId = tab?.id ?? -1;
  const seen = new Set();
  const items = [];
  for (const entry of links.slice(0, LINK_LIMIT)) {
    const link = typeof entry === 'string' ? entry : entry?.link;
    if (!isHttpUrl(link) || seen.has(link)) continue;
    seen.add(link);
    const headers = await collectHeaders({ options, wire: null, url: link, tabId, referer: page });
    items.push(buildItem({ link, headers, downloadPage: page || null, type: isPlaylistUrl(link) ? 'hls' : 'http' }));
  }
  if (items.length === 0) {
    notify(tabId, 'There are no downloadable links here. Only web links can be sent.', 'error');
    return { ok: false };
  }
  return sendItems(items, tabId, options);
}

async function sendMedia(url, tab) {
  const options = await getOptions();
  const item = (media[tab.id] || []).find((m) => m.url === url);
  if (!item) return { ok: false, error: 'That media is no longer on the page. Play it again.' };
  const headers = await collectHeaders({ options, wire: null, url: item.url, tabId: tab.id, referer: tab.url });
  const reply = await sendItems(
    [buildItem({ type: item.type, link: item.url, headers, downloadPage: tab.url, suggestedName: item.name })],
    tab.id,
    options
  );
  return reply;
}

async function collectFromTab(tab) {
  try {
    return (await api.tabs.sendMessage(tab.id, { type: 'collect-links' })) || [];
  } catch {
    return [];
  }
}

async function onMenuClick(info, tab) {
  await ready;
  const options = await getOptions();
  const tabId = tab?.id ?? -1;
  if (info.menuItemId === 'sdm-link') {
    const link = info.linkUrl || info.srcUrl;
    if (!isHttpUrl(link)) {
      notify(tabId, 'Seamless can only take web links. Let the browser save this one.', 'error');
      return;
    }
    await sendLinks([{ link }], tab, options, info.pageUrl);
  } else if (info.menuItemId === 'sdm-selection') {
    const links = await collectFromTab(tab);
    if (links.length === 0) {
      notify(tabId, 'No links in the selection. Select some text that contains links first.', 'error');
      return;
    }
    await sendLinks(links, tab, options, info.pageUrl);
  }
}

let menuChain = Promise.resolve();
function setupMenus() {
  menuChain = menuChain.then(async () => {
    try {
      await api.contextMenus.removeAll();
      const options = await getOptions();
      if (!options.contextMenu) return;
      const quiet = () => void api.runtime.lastError;
      api.contextMenus.create({ id: 'sdm-link', title: 'Download with Seamless', contexts: ['link', 'audio', 'video', 'image'] }, quiet);
      api.contextMenus.create({ id: 'sdm-selection', title: 'Download selected links', contexts: ['selection'] }, quiet);
    } catch {}
  });
  return menuChain;
}

async function handleMessage(message, sender) {
  switch (message?.type) {
    case 'bypass':
      bypass.since = message.down ? Date.now() : 0;
      bypassState.touch();
      return { ok: true };
    case 'status':
      badge('');
      return protocol.status();
    case 'test':
      return protocol.test({ port: message.port, key: message.key });
    case 'send-links': {
      await ready;
      return sendLinks(message.links || [], sender.tab, await getOptions(), sender.tab?.url);
    }
    case 'send-media':
      await ready;
      return sender.tab ? sendMedia(message.url, sender.tab) : { ok: false };
    case 'get-media':
      await ready;
      return { items: sender.tab ? media[sender.tab.id] || [] : [] };
    default:
      return { ok: false };
  }
}

api.webRequest.onSendHeaders.addListener(
  recordRequest,
  { urls: URLS, types: ['main_frame', 'sub_frame', 'other'] },
  isFirefox ? ['requestHeaders'] : ['requestHeaders', 'extraHeaders']
);

if (isFirefox) {
  api.webRequest.onHeadersReceived.addListener(
    onHeadersReceivedBlocking,
    { urls: URLS, types: ['main_frame', 'sub_frame'] },
    ['blocking', 'responseHeaders']
  );
} else {
  api.webRequest.onHeadersReceived.addListener(
    recordResponse,
    { urls: URLS, types: ['main_frame', 'sub_frame', 'other'] },
    ['responseHeaders']
  );
}

api.webRequest.onResponseStarted.addListener(
  handleMedia,
  { urls: URLS, types: ['media', 'xmlhttprequest'] },
  ['responseHeaders']
);

api.downloads.onCreated.addListener((item) => {
  handleCreated(item).catch(() => {});
});

api.downloads.onChanged.addListener((delta) => {
  if (!delta.state) return;
  if (delta.state.current === 'complete') {
    ready.then(() => finishHandoff(delta.id));
  } else if (delta.state.current === 'interrupted' && watch[delta.id]) {
    delete watch[delta.id];
    watchState.touch();
    syncKeepAlive();
  }
});

api.tabs.onCreated.addListener((tab) => {
  newTabs[tab.id] = { t: Date.now() };
  tabsState.touch();
});

api.tabs.onRemoved.addListener((tabId) => {
  delete newTabs[tabId];
  delete media[tabId];
  storeIds.delete(tabId);
  tabsState.touch();
  mediaState.touch();
});

api.tabs.onUpdated.addListener((tabId, change) => {
  if (change.status === 'loading' && change.url && media[tabId]?.length) {
    media[tabId] = [];
    mediaState.touch();
    pushMedia(tabId);
  }
});

api.contextMenus.onClicked.addListener((info, tab) => {
  onMenuClick(info, tab).catch(() => {});
});

api.runtime.onMessage.addListener((message, sender, sendResponse) => {
  handleMessage(message, sender).then(sendResponse, (error) => {
    sendResponse({ ok: false, error: String(error?.message || error) });
  });
  return true;
});

store.onChange((next) => {
  const menuChanged = optionsCache?.contextMenu !== next.contextMenu;
  optionsCache = next;
  if (menuChanged) setupMenus();
});

api.runtime.onInstalled.addListener(() => setupMenus());
api.runtime.onStartup.addListener(() => setupMenus());
if (isFirefox) setupMenus();
