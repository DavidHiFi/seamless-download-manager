# Seamless Download Manager: design

An all-Rust Windows download manager, owned end to end: engine, API, native window,
browser extension. Decisions on 2026-11-22: all-Rust window, our own extension with the
capture gaps fixed. Public since 0.1.0 (2026-11-23), with the Windhawk mod in Seamless
Workshop as the main way to run it. Since 0.1.3 it has no import from other apps.

## Layout

| Path | What |
|---|---|
| `crates/sdm-engine` | Download engine: probe, multi-part ranged download, dynamic split, resume, retry with backoff, speed limits, sparse files, HLS. No storage, no UI. |
| `crates/sdm-core` | Model, SQLite store, settings, categories, queues and scheduler, the `Manager` that ties engine and store. |
| `crates/sdm-api` | Local HTTP API (axum) on 127.0.0.1, protocol types shared with the native host and CLI. |
| `app` | `seamless-download-manager.exe`: Slint window, tray, single instance, autostart, CLI subcommands. |
| `host` | `sdm-host.exe`: browser native messaging host. Tiny, starts the app when it is closed. |
| `extension` | One source for Chromium (MV3) and Zen/Firefox (MV2). Plain JS, no build step. |

Data: `%LOCALAPPDATA%\SeamlessDownloadManager\` (`sdm.db`, `settings.json`, `logs\`).
Binaries: `%LOCALAPPDATA%\SeamlessDownloadManager\bin\` (outside the source folder, so a
build never fights a running copy).

## Design choices

- One process, async IO, no JVM. No second process per CLI or browser call.
- First probe request is kept as part 0, so one-time links are not spent twice.
- Retries back off (1, 2, 4 ... 30 s) and resume uses `If-Range` with the saved ETag or
  Last-Modified, so a changed file restarts instead of corrupting.
- Explicit connect and read timeouts. HTTP/2 allowed.
- SQLite with WAL instead of one JSON per download rewritten every second. Progress
  saved every 2 s, only for running items.
- Cookies and headers stored encrypted with Windows DPAPI (per user), not plain text.
- API key always required, compared in constant time, and handed to the extension by the
  native host, so nobody types it.
- Duplicate sends dropped by redirect chain plus a 15 s window in the extension and in
  the app; a duplicate file becomes `name_1.ext`, never a prompt.
- Capture decided from the response (attachment, MIME), not only a file-type list; web
  pages never captured; send first, cancel the browser download only after the app
  accepted; failed sends stay in the browser; POST, blob and data downloads handed over
  after the browser finishes (the old handoff, folded in).

## Protocol

HTTP on `127.0.0.1:15161`. Header `X-Api-Key`.

| Method | Path | Body | Reply |
|---|---|---|---|
| POST | `/ping` | none | `{"ok":true,"app":"sdm","version":"x"}` |
| POST | `/add` | `{"items":[{type,link,headers,downloadPage,suggestedName}],"options":{silentAdd,silentStart}}` | `{"ok":true,"ids":[..]}` |
| GET | `/queues` | none | `[{"id","name"}]` |
| POST | `/start-headless-download` | one download task (link, folder, name, headers) | `{"ok":true,"id":n}` |
| GET | `/v1/downloads` | none | list with status and progress |
| POST | `/v1/downloads/{id}/pause`, `/resume`, `DELETE /v1/downloads/{id}?file=1` | | `{"ok":true}` |
| POST | `/v1/handoff` | `{"path":"<finished browser file>","url"?,"referrer"?}` | `{"ok":true,"id":n}` adopts the file into the list, no re-download |

Errors: `{"ok":false,"error":"plain words"}` with a 4xx or 5xx code.

Native host `com.seamless.download_manager` (stdio, 4-byte little-endian length, UTF-8 JSON):
request `{"id":"..","action":"ping|add|handoff|config","payload":{..}}`, reply
`{"id":"..","ok":true,"result":..}` or `{"id":"..","ok":false,"error":".."}`. `config` returns
`{"port","apiKey"}` so the extension can talk HTTP directly. The host starts the app with
`--background` when ping fails and waits up to 10 s.

Extension ids: Chromium fixed by the `key` in the manifest; Firefox
`sdm@seamless.local`.

## Look

The rice skill (Catppuccin Mocha, blue to mauve, FiraCode Nerd Font, native title bar).
Window body is DWM acrylic with a Mocha tint tuned by `glass-check.py`; in-window pop-ups
(menus, dropdowns, tooltips) cannot blur in Slint, so they are opaque `#1E1E2E` per the
glass rule. Window edges belong to Seamless Borders, so the app draws none.
