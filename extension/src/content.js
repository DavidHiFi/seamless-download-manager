(() => {
  const api = globalThis.browser ?? globalThis.chrome;
  if (!api?.runtime?.id || window.top !== window || window.__sdmContent) return;
  window.__sdmContent = true;

  let pageButtons = true;
  let bypassKey = 'delete';
  let mediaItems = [];
  let host = null;
  let root = null;
  let pill = null;
  let overlay = null;
  let toastBox = null;
  let pillTimer = null;
  let mouse = { x: 24, y: 24 };
  let bypassDown = false;
  let overlayCollapsed = false;
  let overlayClosed = false;
  let frame = 0;

  const CSS = `
    :host { all: initial; }
    * { box-sizing: border-box; font-family: "FiraCode Nerd Font", "Fira Code", monospace; }
    .glass {
      position: relative;
      color: #CDD6F4;
      background: #1E1E2E;
      border-radius: 12px;
    }
    @supports (backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px)) {
      .glass {
        background: rgb(24 24 37 / .55);
        -webkit-backdrop-filter: blur(22px) saturate(1.25);
        backdrop-filter: blur(22px) saturate(1.25);
      }
    }
    .glass::before, .glass::after {
      content: "";
      position: absolute;
      inset: 0;
      padding: 2px;
      border-radius: inherit;
      pointer-events: none;
      -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
      -webkit-mask-composite: xor;
      mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
      mask-composite: exclude;
      transition: opacity 150ms ease;
    }
    .glass::before { background: rgb(69 71 90 / .9); }
    .glass::after { background: linear-gradient(45deg, #89B4FA, #CBA6F7); opacity: 0; }
    .glass:hover::after { opacity: 1; }
    button {
      all: unset;
      cursor: pointer;
      color: inherit;
      font-size: 14px;
      line-height: 1.3;
    }
    button:focus-visible { outline: 1px solid rgb(137 180 250 / .75); outline-offset: 2px; border-radius: 8px; }
    .pill {
      position: fixed;
      z-index: 2147483647;
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 8px 8px 8px 14px;
      border-radius: 999px;
      font-size: 14px;
    }
    .pill:active::after { background: #CBA6F7; opacity: 1; }
    .pill .go { padding: 2px 4px; }
    .pill .x { width: 24px; height: 24px; display: grid; place-items: center; border-radius: 999px; color: #A6ADC8; }
    .pill .x:hover { background: #313244; color: #CDD6F4; }
    .overlay {
      position: fixed;
      z-index: 2147483646;
      width: 340px;
      max-width: calc(100vw - 16px);
      font-size: 14px;
    }
    .overlay header { display: flex; align-items: center; gap: 8px; padding: 10px 10px 10px 14px; }
    .overlay header .title { flex: 1; font-weight: 600; }
    .overlay .icon { width: 28px; height: 28px; display: grid; place-items: center; border-radius: 8px; color: #A6ADC8; }
    .overlay .icon:hover { background: #313244; color: #CDD6F4; }
    .list { max-height: 240px; overflow: auto; padding: 0 8px 8px; display: grid; gap: 6px; }
    .item {
      display: grid;
      grid-template-columns: 1fr auto;
      align-items: center;
      gap: 4px 10px;
      padding: 8px 10px;
      border-radius: 8px;
      background: rgb(49 50 68 / .6);
    }
    .item .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .item .meta { grid-column: 1; color: #A6ADC8; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .item .send {
      grid-row: 1 / span 2;
      grid-column: 2;
      padding: 6px 12px;
      border-radius: 8px;
      background: #89B4FA;
      color: #11111B;
      font-weight: 600;
    }
    .item .send:hover { background: #B4BEFE; }
    .item .send:active { background: #CBA6F7; }
    .toasts { position: fixed; z-index: 2147483647; right: 16px; bottom: 16px; display: grid; gap: 8px; justify-items: end; }
    .toast { padding: 10px 16px; max-width: min(380px, calc(100vw - 32px)); font-size: 14px; line-height: 1.4; }
    .toast.ok { color: #A6E3A1; }
    .toast.error { color: #F9E2AF; }
    [hidden] { display: none !important; }
  `;

  function ensureRoot() {
    if (root) return;
    host = document.createElement('div');
    host.id = 'sdm-root';
    host.style.cssText = 'all: initial; position: fixed; inset: 0 auto auto 0; width: 0; height: 0; z-index: 2147483647;';
    root = host.attachShadow({ mode: 'closed' });
    const style = document.createElement('style');
    style.textContent = CSS;
    toastBox = document.createElement('div');
    toastBox.className = 'toasts';
    toastBox.setAttribute('role', 'status');
    toastBox.setAttribute('aria-live', 'polite');
    root.append(style, toastBox);
    (document.documentElement || document.body).appendChild(host);
  }

  function svg(path) {
    return `<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${path}</svg>`;
  }

  function toast(text, kind) {
    ensureRoot();
    const el = document.createElement('div');
    el.className = `glass toast ${kind === 'ok' ? 'ok' : 'error'}`;
    el.textContent = text;
    toastBox.appendChild(el);
    setTimeout(() => el.remove(), kind === 'ok' ? 3500 : 7000);
  }

  function selectionLinks() {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return [];
    const found = new Map();
    for (let i = 0; i < selection.rangeCount; i += 1) {
      const range = selection.getRangeAt(i);
      let container = range.commonAncestorContainer;
      if (container.nodeType !== 1) container = container.parentElement;
      if (!container) continue;
      const anchors = new Set(container.querySelectorAll('a[href]'));
      const own = container.closest('a[href]');
      if (own) anchors.add(own);
      for (const a of anchors) {
        let touches = false;
        try {
          touches = range.intersectsNode(a);
        } catch {}
        if (!touches || !/^https?:/i.test(a.href) || found.has(a.href)) continue;
        found.set(a.href, { link: a.href, text: (a.innerText || a.textContent || '').trim().slice(0, 200) });
      }
    }
    return [...found.values()];
  }

  function removePill() {
    clearTimeout(pillTimer);
    pill?.remove();
    pill = null;
  }

  function showPill() {
    if (!pageButtons) return;
    const links = selectionLinks();
    if (links.length === 0) return removePill();
    ensureRoot();
    removePill();
    pill = document.createElement('div');
    pill.className = 'glass pill';
    const go = document.createElement('button');
    go.className = 'go';
    go.textContent = links.length > 1 ? `Download ${links.length} selected links` : 'Download selected link';
    const close = document.createElement('button');
    close.className = 'x';
    close.setAttribute('aria-label', 'Close');
    close.innerHTML = svg('<path d="M3 3l8 8M11 3l-8 8"/>');
    pill.append(go, close);
    root.appendChild(pill);
    const box = pill.getBoundingClientRect();
    const x = Math.min(Math.max(8, mouse.x + 12), window.innerWidth - box.width - 8);
    const y = Math.min(Math.max(8, mouse.y + 16), window.innerHeight - box.height - 8);
    pill.style.left = `${x}px`;
    pill.style.top = `${y}px`;
    go.addEventListener('click', () => {
      removePill();
      api.runtime.sendMessage({ type: 'send-links', links }).catch(() => {});
    });
    close.addEventListener('click', removePill);
  }

  function largestMedia() {
    let best = null;
    let area = 0;
    for (const el of document.querySelectorAll('video, audio')) {
      const r = el.getBoundingClientRect();
      const a = r.width * r.height;
      if (a > area && r.bottom > 0 && r.top < window.innerHeight) {
        area = a;
        best = r;
      }
    }
    return best;
  }

  function placeOverlay() {
    frame = 0;
    if (!overlay) return;
    const r = largestMedia();
    const w = overlay.offsetWidth || 340;
    const h = overlay.offsetHeight || 48;
    let x = 8;
    let y = 8;
    if (r) {
      x = Math.min(Math.max(8, r.left + 8), window.innerWidth - w - 8);
      y = Math.min(Math.max(8, r.top - h - 6 >= 8 ? r.top - h - 6 : r.top + 8), window.innerHeight - h - 8);
    }
    overlay.style.left = `${x}px`;
    overlay.style.top = `${y}px`;
  }

  function queuePlace() {
    if (!overlay || frame) return;
    frame = requestAnimationFrame(placeOverlay);
  }

  function describe(item) {
    const parts = [];
    if (item.duration) {
      const s = Math.round(item.duration);
      const h = Math.floor(s / 3600);
      const m = Math.floor((s % 3600) / 60);
      const sec = String(s % 60).padStart(2, '0');
      parts.push(h ? `${h}:${String(m).padStart(2, '0')}:${sec}` : `${m}:${sec}`);
    }
    if (item.size > 0) {
      const units = ['B', 'KB', 'MB', 'GB'];
      let v = item.size;
      let i = 0;
      while (v >= 1024 && i < units.length - 1) {
        v /= 1024;
        i += 1;
      }
      parts.push(`${v >= 100 || i === 0 ? Math.round(v) : v.toFixed(1)} ${units[i]}`);
    }
    if (item.resolution) parts.push(item.resolution);
    if (item.bandwidth) parts.push(`${Math.round(item.bandwidth / 1000)} kbps`);
    return parts.join('   ');
  }

  function renderOverlay() {
    if (!pageButtons || overlayClosed || mediaItems.length === 0) {
      overlay?.remove();
      overlay = null;
      return;
    }
    ensureRoot();
    if (!overlay) {
      overlay = document.createElement('div');
      overlay.className = 'glass overlay';
      root.appendChild(overlay);
    }
    overlay.replaceChildren();
    const head = document.createElement('header');
    const title = document.createElement('span');
    title.className = 'title';
    title.textContent = `Download with Seamless (${mediaItems.length})`;
    const fold = document.createElement('button');
    fold.className = 'icon';
    fold.setAttribute('aria-label', overlayCollapsed ? 'Show the list' : 'Hide the list');
    fold.innerHTML = svg(overlayCollapsed ? '<path d="M3 5l4 4 4-4"/>' : '<path d="M3 9l4-4 4 4"/>');
    const close = document.createElement('button');
    close.className = 'icon';
    close.setAttribute('aria-label', 'Close');
    close.innerHTML = svg('<path d="M3 3l8 8M11 3l-8 8"/>');
    head.append(title, fold, close);
    overlay.appendChild(head);
    fold.addEventListener('click', () => {
      overlayCollapsed = !overlayCollapsed;
      renderOverlay();
    });
    close.addEventListener('click', () => {
      overlayClosed = true;
      renderOverlay();
    });
    if (!overlayCollapsed) {
      const list = document.createElement('div');
      list.className = 'list';
      for (const item of mediaItems) {
        const row = document.createElement('div');
        row.className = 'item';
        const name = document.createElement('span');
        name.className = 'name';
        name.textContent = item.name;
        name.title = item.name;
        const meta = document.createElement('span');
        meta.className = 'meta';
        meta.textContent = describe(item);
        const send = document.createElement('button');
        send.className = 'send';
        send.textContent = 'Download';
        send.addEventListener('click', () => {
          api.runtime.sendMessage({ type: 'send-media', url: item.url }).catch(() => {});
        });
        row.append(name, meta, send);
        list.appendChild(row);
      }
      overlay.appendChild(list);
    }
    placeOverlay();
  }

  function setMedia(items) {
    mediaItems = Array.isArray(items) ? items : [];
    if (mediaItems.length === 0) overlayClosed = false;
    renderOverlay();
  }

  function applyOptions(options) {
    pageButtons = options?.pageButtons !== false;
    bypassKey = (typeof options?.bypassKey === 'string' && options.bypassKey ? options.bypassKey : 'Delete').toLowerCase();
    if (!pageButtons) {
      removePill();
      renderOverlay();
    } else {
      renderOverlay();
    }
  }

  function setBypass(down) {
    if (bypassDown === down) return;
    bypassDown = down;
    api.runtime.sendMessage({ type: 'bypass', down }).catch(() => {});
  }

  document.addEventListener('mousemove', (e) => {
    mouse = { x: e.clientX, y: e.clientY };
  }, { passive: true });

  document.addEventListener('mouseup', (e) => {
    if (e.target === host) return;
    clearTimeout(pillTimer);
    pillTimer = setTimeout(showPill, 500);
  }, true);

  document.addEventListener('mousedown', (e) => {
    if (e.target !== host) removePill();
  }, true);

  document.addEventListener('keydown', (e) => {
    if (e.key.toLowerCase() === bypassKey) setBypass(true);
    else if (e.key === 'Escape') removePill();
  }, true);
  document.addEventListener('keyup', (e) => {
    if (e.key.toLowerCase() === bypassKey) setBypass(false);
  }, true);
  window.addEventListener('blur', () => setBypass(false));
  window.addEventListener('scroll', queuePlace, { passive: true, capture: true });
  window.addEventListener('resize', queuePlace, { passive: true });

  api.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    switch (message?.type) {
      case 'toast':
        toast(String(message.text || ''), message.kind);
        break;
      case 'collect-links':
        sendResponse(selectionLinks());
        break;
      case 'media':
        setMedia(message.items);
        break;
      default:
        break;
    }
  });

  api.storage.onChanged.addListener((changes, area) => {
    if (area === 'local' && changes.options) applyOptions(changes.options.newValue);
  });

  api.storage.local.get('options').then((stored) => applyOptions(stored?.options)).catch(() => {});
  api.runtime.sendMessage({ type: 'get-media' }).then((reply) => setMedia(reply?.items)).catch(() => {});
})();
