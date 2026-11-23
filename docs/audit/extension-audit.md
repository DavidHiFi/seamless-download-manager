# AB Download Manager browser extension audit

Audited 2026-11-22, read-only. Source is the shipped, minified bundles, so names like `Hu`, `Ya()` and `tT()` are the minifier's, not the authors'. No keys, cookies or tokens were read or printed. Stored option values were not read, only option names from the code.

Paths used below:

- HEL = `%LOCALAPPDATA%\imput\Helium\User Data\Default\Extensions\bbobopahenonfdgjgaleledndnnfhooj\1.5.2_0\`
- ZEN = `%APPDATA%\zen\Profiles\<profile>\extensions\firefox-integration@abdownloadmanager.com.xpi` (a zip, 1.5.2)
- BG = `assets/src/entrypoint/Background-260b06d1.js`
- DEF = `assets/DefineExtensionEntry-d9f2f045.js`
- CS = `assets/src/entrypoint/ContentScript-0cff1f94.js`
- OPT = `assets/src/entrypoint/OptionUi/index-fdf704d6.js`
- POP = `assets/src/entrypoint/BrowserAction/index-0c002c2b.js`

## 1. Versions, manifest, structure

| | Helium (Chromium) | Brave (Chromium) | Zen (Firefox) |
|---|---|---|---|
| Id | `bbobopahenonfdgjgaleledndnnfhooj` | same | `firefox-integration@abdownloadmanager.com` |
| Version | 1.5.2 | not on disk now | 1.5.2 |
| Manifest | MV3 | MV3 | MV2 |
| Background | service worker `serviceWorker.js`, module | same | persistent page `background.html` |
| Can block a response | no | no | yes (`webRequestBlocking`) |
| Popup | `src/entrypoint/BrowserAction/index.html` | same | same (`browser_action`) |
| Options | `src/entrypoint/OptionUi/index.html`, in a tab | same | same |
| Locales | `en` only (about 60 strings) | | `en` |

Brave: its Extensions folder holds no copy of this id right now. The policy keys (ExtensionInstallForcelist, HKLM and HKCU) still list it, and the 2026-10-04 note says 1.5.1 was installed that way. Unexplained, reported as is. Assume the same build as Helium, unverified.

Permissions (Chromium): contextMenus, webRequest, cookies, storage, tabs, downloads, notifications, nativeMessaging, plus host `*://*/*`. Zen adds webRequestBlocking. No declarativeNetRequest, scripting or alarms.

Content script: one, `ContentScript.js` on `*://*/*` (top frame). Real code is in CS. Web-accessible: ContentScript, DEF, `Shortcut-4461e19e.js`, one shared chunk.

Stack: Vite, React, mobx, DaisyUI, zod, rxjs, lodash, hls-parser, a webext-bridge style message bus, webextension-polyfill.

Keep-alive (MV3 only): `setInterval(chrome.runtime.getPlatformInfo, 10000)`, restarted on `onStartup`. That is the whole defence against the worker sleeping.

Native hosts on this machine, in `%USERPROFILE%\.abdm\config\native_messaging\`:

- `chrome\com.abdownloadmanager.json`: allowed origin is `chrome-extension://bbobopahenonfdgjgaleledndnnfhooj/` only.
- `firefox\com.abdownloadmanager.json`: allowed extension is `firefox-integration@abdownloadmanager.com` only.
- `handoff\com.local.abdm_handoff.json` (ours): allowed origin `chrome-extension://oodniaigpjkfedcencfpbafinanfhgom/`.

Both official ids are hard-coded in the app (`shared/app/build.gradle.kts` and its shared constants). A new SDM id is refused by the native host until it is added there and the manifests are re-applied. Firefox needs a fixed `browser_specific_settings.gecko.id`; Chromium needs a fixed `key` in the manifest so the id never changes.

## 2. Every option and feature

One zod schema in DEF validates config in `storage.local`. A bad value falls back to its default. `storage.local.onChanged` reloads config everywhere.

| Key | Default | UI label | What it does |
|---|---|---|---|
| `autoCaptureLinks` | true | Auto Capture Links | Master switch for taking downloads from the browser. |
| `registeredFileTypes` | zip rar 7z iso tar gz exe msi deb jar apk bin mp3 aac pdf mp4 3gp avi mkv wav mpeg srt | text area | Lowercase extensions, exact string match. Our Helium copy holds 361. |
| `blacklistedUrls` | [] | Ignored Url patterns | One pattern per line, `*` wildcard. Checked against origin URL, final URL and download page. |
| `captureFileSizeMinimumKb` | 0 | Capture file size minimum (KB) | Smaller files stay in the browser. Unknown size is accepted. |
| `bypassShortcut` | Delete (Backspace on macOS) | Bypass Shortcut | Hold while clicking to let the browser download. |
| `popupEnabled` | true | Show popups | Selection popup and media overlay. Also gates media detection. |
| `contextMenuEnabled` | true | Add To Context Menu | Creates the two menu items. |
| `silentAddDownload` | false | Add downloads silently | App adds without its dialog. Shows a "Download captured silently" notice for a single item. |
| `silentStartDownload` | false | Start download silently | Needs silent add. |
| `sendHeaders` | true | Send Cookies | Attach headers and cookies to menu and selection items. |
| `port` | 15151 | Port + Test | App HTTP port, 1024 to 65535. |
| `apiKey` | "" | API Key | Sent as `X-Api-Key` when not empty. |
| `allowPassDownloadIfAppNotRespond` | true | none | Let the browser download if the app cannot be reached. |
| `closeNewTabIfItWasCaptured` | true | none | Close the empty tab a download link opened. |

Native messaging is not an option. It is detected at boot and shown as "Host Is Reachable" or "Host Is Not Reachable".

Other features:

- Context menu `download-with-ab-dm` (link, audio, video, image) and `download-selected-with-ab-dm` (selection).
- Selection popup: a pill "Download selected" shown 500 ms after mouseup when the selection holds links.
- Media overlay "Download With ABDM (n)" above the largest video or audio element.
- Toolbar popup with three toggles (section 6).
- Failures show a browser `alert()` in the active tab.
- Media detection is off on `youtu.be`, `youtube.com`, `*.youtube.com`.

## 3. Capture logic

All in BG, class `Hu`, method `redirectDownloadsToExtension()`. Chromium uses subclass `MT` (cannot block), Zen uses `XT` (can block). Chosen in `tE()`.

### 3.1 Bookkeeping

- `tabs.onCreated/onUpdated/onRemoved` keep `tabCache[tabId]={url,title}` (download page fallback) and `newTabs` (to close the empty tab after capture).
- `webRequest.onSendHeaders` (`*://*/*`, request headers, `extraHeaders` on Chromium) stores `pendingRequests[requestId]={initialRequest, finalRequest}`. Redirects overwrite `finalRequest`. This is where the real request headers come from.
- `onErrorOccurred` and `onCompleted` delete the entry after 20 s.

### 3.2 Path A: `webRequest.onHeadersReceived`

1. No pending entry: pass.
2. `shouldHandleRequestForDirectDownload` needs all of:
   - type `main_frame` or `sub_frame`, method `GET`
   - `autoCaptureLinks` on, status 200 to 299
   - `Content-Type` not starting with `text/`
   - origin URL, URL and download page not blacklisted
   - `Content-Length` not below the minimum
   - bypass key not held
   - `isDirectDownloadContent`: file name from `Content-Disposition` (`filename*=UTF-8''...` first, then `filename=`, RFC 2047 decoded), else the last URL path segment; take text after the last dot; accept only if exactly in `registeredFileTypes`.
3. Pass: set `handledOnWebRequest=true`, build the item, send to the app.
4. Accepted: close the new tab if allowed, and on Zen return `{cancel:true}`. On Chromium nothing can be cancelled here, so path B cancels the browser download.
5. Failed: if `allowPassDownloadIfAppNotRespond`, return the response unchanged. `onDownloadSendToAppFailed` also resets `handledOnWebRequest=false`.

### 3.3 Path B: `downloads.onCreated`

1. Skip if capture is off, URL not http(s), or `byExtensionId` is set.
2. Skip if start time is over 10 s away or it already has an end time.
3. Find the pending request by URL (first or last hop). No match: skip. `shouldSkipUninterceptedDownloads()` is always true, so `blob:`, `data:` and anything not seen on the wire is never captured.
4. Already handled in path A: cancel, erase and remove the browser download, done.
5. Method not GET: stop. POST stays in the browser.
6. Re-check blacklist, size (`fileSize`) and bypass key, using the download page or the download's `referrer`.
7. Headers: stored request headers, else `cookies.getAll({url})` (3.5).
8. Re-run `isDirectDownloadContent` on the response; unlisted types stay in the browser.
9. Cancel, erase and remove the browser download, then send. If the send fails the download is already gone.

### 3.4 Other entry points

- Menu "Download with AB DM": `linkUrl` or `srcUrl`, page URL as `downloadPage`, link text as `description`, headers from cookies if `sendHeaders`. No type or blacklist check.
- Menu "Download selected": asks the content script of the active tab.
- Selection (`N()` in CS): per selection range, take the common ancestor, collect every `<a href>` touching the range, item `{link, headers:null, description: innerText, downloadPage: location.href, type:"http"}`. The background fills headers (`GET_HEADERS`) if `sendHeaders`. Alerts "Please first make a selection" and "No links detected".
- Media:
  - `onResponseStarted` type `media` with `Content-Type` starting `video` or `audio`: direct media.
  - `xmlhttprequest` URLs matching `*.m3u8` (with or without query): HLS. Playlist is fetched (2 MB cap, content-type check), parsed with hls-parser, master playlists recurse into variants, encrypted playlists are dropped, only `.ts` segments are offered.
  - Names: `Content-Disposition`, else URL, else tab title plus a short hash, with resolution and frame rate. A MIME to extension table fills a missing extension.
  - Needs `popupEnabled`, page not blacklisted, not YouTube, size at or above the minimum.
  - The list is pushed to the tab's content script (`DOWNLOADABLE_MEDIA_DETECTED`) and reset when the tab loads or the URL changes.

### 3.5 Headers, cookies, referrer

- Wire path (A and B): every request header the browser sent, as seen at `onSendHeaders`. On Chromium `extraHeaders` makes Cookie, Referer and Origin visible.
- Fallback `Ya(url)`: `{Cookie: "n=v; ...", Host, User-Agent}` from `cookies.getAll({url})`. No Referer.
- `sendHeaders` is checked on the menu and selection paths only. Paths A and B attach wire headers regardless.
- Referrer goes as `downloadPage` (document URL, tab URL or `download.referrer`) plus any Referer header.

### 3.6 When the browser keeps the download

Not GET, not main or sub frame, response is `text/*`, URL or page blacklisted, below the size minimum, bypass key held, extension not in the list (or no extension), app unreachable (path A), no matching request seen, or started by another extension.

## 4. Protocol to the app

### 4.1 Transport choice

One client (`jv`) with two transports: HTTP and native. At boot `tT()` pings the native host once. If it worked, native is preferred, otherwise HTTP. Errors shown come from the preferred transport.

### 4.2 HTTP

- Base `http://localhost:{port}/`. The app (Ktor CIO) listens on port 15151 by default.
- Port discovery: none. It is a setting. The options Test button pings the typed port.
- Auth: header `X-Api-Key`, sent only when the key is not empty. The app enforces it only when "API key enforced" is on. Wrong key gives 401.
- Timeout 500 ms (AbortController). Failure is shown as a network error.
- `POST /ping`, answer `pong`.
- `POST /add`, answer `OK`:

```json
{
  "items": [
    {
      "type": "http",
      "link": "https://host/file.zip",
      "headers": { "Cookie": "...", "User-Agent": "...", "Referer": "..." },
      "downloadPage": "https://host/page",
      "description": null,
      "suggestedName": null
    }
  ],
  "options": { "silentAdd": false, "silentStart": false }
}
```

  `type` is `"http"` or `"hls"`. `headers` can be null. The app model ignores `description`. Media items carry `suggestedName` with extension. The app still accepts a legacy bare array of items. `silentStart` is only true when `silentAdd` is.
- Unused app endpoints: `GET /queues`, and `POST /start-headless-download` (key always required) with `{downloadSource:{type,link,headers,downloadPage,suggestedName}, folder, name, queueId, categoryId, startDownload, startQueue}`. `REST-API.yml` does not document the `items/options` wrapper or `type`.

### 4.3 Native messaging (`com.abdownloadmanager`)

- Host `ABDownloadManagerNativeMessagingHost.exe`, stdio, 4-byte native-endian length then UTF-8 JSON, 4 MB cap.
- Persistent `connectNative` port when connected, otherwise `sendNativeMessage` per request. 10 s timeout.
- Request (the payload is a JSON string inside JSON):

```json
{ "id": "B_<uuid>", "content": { "action": "add", "payload": "<same JSON as HTTP /add>", "isError": false } }
```

  `action` is `add` or `ping`.
- Reply: `{ "id": "<same>", "content": { "action": null, "payload": "<json>", "isError": false } }`. On failure `isError` is true and payload is a JSON string `{"errorType":"...","message":"..."}`.
- Waking the app: the host runs `EnsureAppIsAwake.awakeTheApp(timeout=10000)`. If the app is not ready it starts `<cli exe> gui start-if-not-started`, waits up to 10 s, then forwards over the app's single-instance IPC. A failure raises "Can't awake the app". This is why native add works with the app closed and HTTP does not.

### 4.4 Our handoff (`com.local.abdm_handoff`), not part of the official extension

- Chromium: unpacked MV3 "AB completed download handoff" 1.0.0, permissions downloads, nativeMessaging, storage, fixed id `oodniaigpjkfedcencfpbafinanfhgom`. On a completed download (from `onCreated` or `onChanged`) that exists and is safe, it opens `connectNative("com.local.abdm_handoff")` and posts `{"filename":"<absolute path>"}`. Reply `{"ok":true,"id":...}` or `{"ok":false,"restored":true,"error":"<ExceptionName>"}`, stored as `lastResult` in `storage.local`.
- Zen: no native messaging. `abdm-handoff.sys.mjs` (loaded by `install-zen.ps1` through the Sine loader and a patched `config.js`) uses the Downloads API and starts `abdm-handoff.exe --file <path>` hidden for each newly succeeded download.
- Host `abdm-handoff.exe` (`scripts\abdm-handoff\bridge.py`, 4-byte little-endian length, 64 KB cap), in order:
  - refuse non-regular files, symlinks, and anything under WINDIR, Program Files, LocalAppData or AppData
  - hash with SHA-256
  - rename to `.<name>.abdm-source-<token>`
  - serve once from `http://127.0.0.1:<random>/<token>` with Range and HEAD
  - run `ABDownloadManagerCli.exe download add http --link <url> --folder <dir> --name <name> --start`
  - poll the app's `downloadlist` DB for the token (up to 10 minutes)
  - verify final path and hash, copy Zone.Identifier, delete the source, write a receipt
  - on error or cancel, pause the download and restore the original file
- It adds no speed, because the file is already downloaded. Its job is catching what the official extension skips (blob, data, POST, unlisted types).

## 5. Known gaps and bugs, with fixes for SDM

| # | Gap or bug | Evidence | Fix for SDM |
|---|---|---|---|
| 1 | POST downloads are never captured | Path A needs GET, path B returns on non-GET | Default: leave POST to the browser and let the completed-file handoff take it. Option: add `method` and a body field to the item, which needs an app change. |
| 2 | Extensionless and unknown-extension files escape | `isDirectDownloadContent` is an exact match on the text after the last dot; a name with no dot uses the whole name. There is no `*` wildcard. Seen on Zen 2026-10-04 and Brave 2026-10-05 | Decide on the response: capture when `Content-Disposition` is `attachment`, or the MIME type is not a page or image (`text/html`, `text/plain`, `application/json`, `image/*`, configurable), or when the browser itself opens a download. Keep the extension list only as an include or exclude override. |
| 3 | Double send | Path A sends, path B sends again when the URL lookup misses after a redirect. The app shows "already exists". Noted 2026-11-02 | Key every send on the whole redirect chain (all hops of the request id), plus the browser download id and a 15 s window. Drop repeats in the extension, not only in the app. |
| 4 | html, htm, php, xhtml in the type list | Login pages were sent to the app, which refetched without the session and got 401 or 403. Removed by hand on Helium, still possible on Zen | Never capture `text/html` or `application/xhtml+xml`, whatever the list says. Strip those four types from any imported list. |
| 5 | MV3 worker sleep loses state | `pendingRequests`, `tabCache`, `newTabs`, the native ping result and config are in worker memory. Only defence is the 10 s timer | Persist pending requests and the tab cache in `storage.session`. Re-run capture at `downloads.onCreated` from data the download item already has (URL, final URL, referrer, filename, MIME). Re-read config on every wake. Hold a native port open while a transfer is running. |
| 6 | Chromium cannot stop a download before it starts | Path A cannot cancel, so a download bubble or flash can show before path B cancels | Cancel and erase in the first `onCreated` tick, hide the shelf where supported. Zen can block in `onHeadersReceived`. |
| 7 | App failure on path B loses the download | `cancelDownload` runs before the send; if the app is down nothing is restored | Send first, cancel only after the app accepts. On failure, leave the browser download running. |
| 8 | `sendHeaders` ignored on paths A and B | The wire headers are copied unconditionally | Honour the toggle everywhere. Forward an allow-list only (Cookie, Referer, User-Agent, Authorization, Accept-Language, Origin) and drop Range, If-*, Sec-Fetch-* and Host. |
| 9 | Weak fallback headers | `Ya()` has no Referer and `cookies.getAll({url})` has no `storeId`, so containers and private windows get the wrong jar | Add Referer from the page. Pass the tab's store id (Firefox container, incognito). |
| 10 | Selection popup misses a selection inside one text node | `N()` handles element ancestors only | Walk up to the parent element when the common ancestor is a text node. |
| 11 | `blob:`, `data:` and extension-started downloads ignored | `shouldSkipUninterceptedDownloads()` is true | Keep ignoring in the extension. The completed-file handoff covers them. |
| 12 | Two options have no UI | `allowPassDownloadIfAppNotRespond`, `closeNewTabIfItWasCaptured` | Give each a toggle, or remove them. |
| 13 | 500 ms timeout and `localhost` only | A busy or cold app misses the window and the file goes to the browser | Use `127.0.0.1`. About 2 s for `add`, 500 ms for `ping`. Prefer native messaging, because it also wakes the app. |
| 14 | Errors use `alert()`, and the options footer has a heart emoji | BG alert path, OPT footer | Use an in-page toast or a toolbar badge. No emoji. |
| 15 | Extension ids are fixed in the app | SharedConstants, native manifests | In our app fork, add the SDM ids to `allowed_origins` and `allowed_extensions`, re-apply native messaging, before the first test. |
| 16 | The catch-all needs a second extension and Developer mode | Brave note 2026-10-05 | Fold the handoff into SDM so there is one extension, one native host and one id to allow. Fix the Chromium key for a stable id. |
| 17 | `description` is dropped by the app | Not in the app model | Do not send it, or add it to the app model. |
| 18 | `suggestedName` is null on the capture path | Item built with `suggestedName:null` | Send the name from `Content-Disposition`, so the app does not guess. |
| 19 | Brave copy missing on disk | Section 1 | Recheck when Brave runs. Not an extension bug. |

Suggested SDM capture order:

1. Decide from the response, not a name list (fixes 2 and 4). Send first, cancel after the app accepts (fix 7).
2. Record every hop in a persisted map and send once (fixes 3 and 5).
3. If the app refuses, or the request is POST, leave the download in the browser and let the handoff pick it up when it completes.

## 6. UI inventory (for restyling)

Both screens use DaisyUI classes (`bg-base-200`, `checkbox`, `input`, `textarea`, `select`, `btn btn-primary`, `link link-primary`) with a dark theme. No tabs, no Save button. Every change writes to `storage.local` at once.

### Toolbar popup (`w-max`)

1. Header (`p-4 bg-base-200`): 32 px app icon on the left. Right side: title "AB DM Quick Settings" and a status line: "Checking..." (neutral), "Connected" (green), "Not Connected" (red), from `IS_APP_REACHABLE`.
2. Row "Auto Capture Links" with a checkbox.
3. Row "Show popups" with a checkbox.
4. Row "Add downloads silently" with a checkbox.
5. Row "More settings" with a chevron icon, opens the options page (`runtime.openOptionsPage`).

Rows are `px-4 py-4`, hover `bg-primary/10`, and the whole row toggles.

### Options page (`w-96`, centred, one column)

1. Logo (64 px) and title "AB Download Manager Extension".
2. Settings block (`bg-base-200`) with a 32 px icon and "Settings", then sections divided by 1 px lines:
   1. Auto Capture Links: checkbox and text. Inside it:
      - file types text area (space separated, lowercased as typed) with a "Reset to default" link that shows only when changed
      - "Ignored Url patterns" text area (one per line) with its own reset link and help line
      - "Capture file size minimum (KB)": a select (Custom, No limit, 100 MB, 500 MB, 1 GB) and a "Custom value" number input with a "KB" suffix
      - "Bypass Shortcut": read-only key recorder (Esc cancels, Tab leaves), a macOS Control warning, a reset link and a description
   2. Show popups: checkbox.
   3. Add To Context Menu: checkbox.
   4. Add downloads silently: checkbox, and below it, indented, "Start download silently" (disabled until the first is on).
   5. Native Messaging: status text only ("Host Is Reachable" green, "Host Is Not Reachable" amber).
   6. Port: number input and a "Test" button, result shown with `alert()` (Success or Failure plus the port). Below it, an API Key text input.
   7. Send Cookies: checkbox and a "don't use in untrusted environments" note.
3. Footer: a note to install the app first with a "Download The App" link, a heart line asking for a rating, and links "Project Website" and "Project Source Code".

### In-page UI (content script)

- Selection pill (shadow DOM): 999 px radius, gradient `#2E3038` to `#171820`, 1 px border at 25% white, 12 px text, grey `#aaaaaa`, hover 25% white, `z-index 9999`, near the last mouse position, removed on mousedown elsewhere. Contains the app icon, "Download selected" and a close button.
- Media overlay (`.abdm-extension`, plain DOM): fixed at top-left 8 px, or above the largest video. Header with icon, "Download With ABDM (n)", collapse toggle and close X. Each item shows the name and a line "duration, size, resolution, bandwidth, extension". Same gradient and border, `z-index 999999`.
- Native `alert()` boxes for no selection, no links, and connection errors.
- Notification: 128 px icon, title "AB Download Manager", message "Download captured silently".

All strings are English in `_locales/en/messages.json` and are all that must be copied for a first SDM pass.

## Appendix: checked and not checked

Read in full or the relevant regions: both manifests, `_locales/en`, BG (clients, native transport, capture class, media pipeline, tail), DEF, CS, OPT, POP, the two ABDM notes, the app's `Endpoints.kt`, `DownloadCredentialsFromIntegration.kt`, `REST-API.yml`, `EnsureAppIsAwake.kt`, the native manifests, and the handoff files (`bridge.py`, Chromium `background.js` and manifest, `chromium-host.json`, `abdm-handoff.sys.mjs`, `install-zen.ps1`).

Not verified: the Zen bundle was compared by manifest only, and it contains both the blocking and non-blocking classes; the rest is assumed identical to Helium 1.5.2. Stored option values were not read. The app's single-instance IPC protocol was not read beyond the wake call. Nothing was run on a real page; this is a source audit.
