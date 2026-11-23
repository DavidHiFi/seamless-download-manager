# AB Download Manager audit (for the Seamless Download Manager Rust rewrite)

Scope: Windows only. Ignored: android/, mac_utils, Linux-only code, build/. Read-only audit, no AB source was edited.
No keys, cookies or tokens appear here. Where a fact was not read in source it is marked UNVERIFIED.

Path shorthands:
- ENG = `ab-download-manager/downloader/core/src/commonMain/kotlin/ir/amirab/downloader/`
- DESK = `ab-download-manager/desktop/app/src/main/kotlin/com/abdownloadmanager/desktop/`
- SHARED = `ab-download-manager/shared/app/src/commonMain/kotlin/com/abdownloadmanager/shared/`
- INTEG = `ab-download-manager/integration/server/src/main/kotlin/com/abdownloadmanager/integration/`

Local uncommitted edits (KEEP in the rewrite):
1. Custom window style (acrylic blur, blur amount, native title bar): `desktop/app-utils/.../window/custom/WindowStyle.kt` (new), `CustomWindow.kt` (+272), `pages/settings/DesktopSettings.kt` (+43), `DesktopSettingsComponent.kt` (+16), `renderapi/RenderApi.kt` (x64 default DIRECT3D instead of OPENGL, about line 55), `DefaultThemes.kt` (+29, Catppuccin Mocha), `SliderBar.kt` (new), `IntConfigurable*.kt`.
2. 15 s duplicate-link drop: `DESK/integration/IntegrationHandlerImp.kt:29-47`.
3. Numbered copy instead of the "file exists" prompt: `SHARED/pages/adddownload/single/BaseAddSingleDownloadComponent.kt` about 456-460 (`CanAddResult.DownloadAlreadyExists -> setOnDuplicateStrategy(OnDuplicateStrategy.AddNumbered)`).

---

## 1. Feature inventory

### 1.1 Download engine
- Multi-part HTTP(S) download, one worker per part: `ENG/downloaditem/http/HttpDownloadJob.kt:353-433`
- Dynamic part creation (split the biggest running part when a thread frees up): `HttpDownloadJob.kt`, `PartSplitSupport.kt` (SAFE_ZONE 1 MiB)
- Resume via Range / 206: `HttpDownloadJob.kt:631-693`, `HttpResponseInfo`
- Retry with fixed 1 s delay: `PartDownloader.kt` (PART_MAX_TRIES 10), `HttpDownloadJob.kt:460-529`
- Sparse file allocation (NTFS/ReFS): `EmptyFileCreator`, `SparseFile.desktop.kt`
- Global and per-item speed limiter: `SpeedLimiter.kt`, `OkioCustomizedThrottler`
- HLS (m3u8) downloader: `HLSDownloader` registered in `DESK/di/Di.kt:250-255`
- Filename from Content-Disposition (RFC 5987 and RFC 2047), then URL: `fileNameExtractor`, `HttpResponseInfo`
- Server Last-Modified applied to the file (optional): `DownloadSettings.useServerLastModifiedTime`
- Basic auth (item username/password), user agent override, custom headers: `OkHttpHttpDownloaderClient`
- Proxy (system, PAC script, manual HTTP/SOCKS) and custom DNS: `Di.kt:495-738`
- Ignore SSL errors switch: `AppSSLFactoryProvider`, `AppHostNameVerifier` (`Di.kt`)

### 1.2 Queues and scheduler
- Queues with max concurrent (default 2), default queue id 0 "Main": `ENG/queue/QueueManager`, `DownloadQueue`
- Per-queue weekday schedule with start and end times ("HH:mm"), stop-queue-on-empty: `ScheduleTimes`, `DownloadQueue`
- 500 ms debounced "shake" that starts the next items: `DownloadQueue`
- Per-queue power action on finish and per-download action: `extra_queue_settings`, `extra_download_settings`, `DesktopOnQueueEventActionProvider`, `DesktopOnDownloadCompletionActionProvider` (`Di.kt:280-380`)
- Select-queue dialog state saved in `pages/selectQueue.json`

### 1.3 Categories
- Six defaults (ids 0-5): Compressed, Programs, Videos, Music, Pictures, Documents, each with a folder under the default download folder and an extension list: `SHARED/util/category/DefaultCategories.kt:28-151`
- Category model: id, name, icon (uri), path, usePath, acceptedFileTypes, acceptedUrlPatterns (wildcard), items (download ids): `Category.kt:21-40`
- Match by extension suffix or URL wildcard: `Category.kt:51-83`
- Category auto-selection when `useCategoryByDefault` is on; file `download_db/categories/categories.json` (live: `[]`)
- Manager and storage: `CategoryManager.kt` (277 lines, UNVERIFIED in detail)

### 1.4 Add-download flow
- Single add dialog with link check (Range probe), name, folder, category, queue: `BaseAddSingleDownloadComponent.kt` (534 lines; only the duplicate branch read)
- Multi-link / batch add: components under `SHARED/pages/adddownload/multiple/` (UNVERIFIED)
- Clipboard link watcher: UNVERIFIED
- File-exists policy `OnDuplicateStrategy` (AddNumbered, Abort, Overwrite style): `ENG/utils/OnDuplicateStrategy`, numbering `name_N.ext` in `FileNameUtil.numberedIfExists`
- Block add while still loading link info: setting `blockAddWhenLoading`
- Silent add and silent start from the browser: `ImportOptions` / `SilentImportOptions` (`IntegrationHandlerImp.kt:49-60`)
- Local edit: duplicate prompt replaced by numbered copy (see top)

### 1.5 Progress and completion windows
- Per-download progress window, auto-shown when `showDownloadProgressDialog` is on (live: off)
- Completion window when `showDownloadCompletionDialog` is on (live: off)
- Both in `DESK/pages/` (UNVERIFIED line level)

### 1.6 Main list
- Columns (from `pageStatesStorage.json`): #, Name, Size, Status, Speed, Time Left, Date Added, Date Started, Date Finished, Location. Live visible set: first seven.
- Sort by column, ascending or descending (live: Date Added descending), column order, widths and visibility saved
- Category tree on the left with a saved width; queue list
- Toolbar of action buttons with optional labels (`showIconLabels`): `DESK/pages/home/Actions.kt:29-73` (separators, single items, drop-down sub menus, horizontal scroll)
- Filters, search box, context menu items: `HomeComponent.kt` (442 lines), `HomePage.kt` (933 lines), UNVERIFIED in detail
- Relative date/time display switch (`useRelativeDateTime`)

### 1.7 Settings (all keys, see section 4.1 for the schema)
Rows come from `SHARED/settings/CommonSettings.kt` (read in full) plus desktop rows in `DesktopSettings.kt`. Units: threads (TextField), max concurrent downloads (0 means unlimited, `CommonSettings.kt:81-96`), retry count (range 0..MaximumDownloadRetriesLimitation.MAX_ALLOWED_RETRIES, 0 is no retries, `:98-118`), dynamic parts, min part size (range MIN..MAX_ALLOWED_MIN_PART_SIZE, `:131-145`), server Last-Modified, append extension, sparse, track deleted files, delete partial on cancel, ignore SSL, default user agent (blank = disabled), block add while loading, use category by default, size unit (binary or decimal bytes), speed unit (binary/decimal bytes and bits), show completion/progress dialogs, per-host settings (navigates to its own page), speed-limit mode (enum) and value (min 1), average speed vs exact, default download folder (must be writable), UI scale (0.8 0.9 1 1.1 1.25 1.5 1.75 2), theme, default dark theme, default light theme, language (null = system), icon labels, relative date, start on boot, notification sound switch and three sound files (general, error, success), browser integration on/off, port (0..65000), API auth on/off, API key (validity shown), proxy editor, DNS editor.

### 1.8 Tray, notifications, sounds
- System tray icon (`useSystemTray`, default true); native menu bar (`useNativeMenuBar`, default false)
- Notification manager and sounds with default sounds when the path is empty: `NotificationManager` (`Di.kt`), `INotificationSound.DEFAULT_VALUE`. UNVERIFIED in detail.

### 1.9 Single instance, auto-start, updater
- Single instance by file lock `config/app.lock` plus a localhost kRPC websocket (section 3.3): `DESK/utils/singleInstance/`
- Auto-start: `Startup.getStartUpManagerForDesktop(name, path=mainExe, args=[--background])` (`Di.kt` startUpModule); setting `autoStartOnBoot` (default true in shared defaults, live false)
- Updater: `GithubUpdateChecker` (plain OkHttpClient to GitHub releases), `UpdateManager`, `DesktopDirectLinkUpdateApplier` downloading into `system/update/` (`Di.kt:436-476`). AppVersionTracker and PreviousVersion kept in system dir.
- Start arguments: `--background`, `--debug`; commands gui, run, exit, start-if-not-started.

### 1.10 Themes, fonts, UI scale
- ThemeManager with selectable dark and light themes, default dark and light ids; live theme `catppuccin_mocha` (local addition in `DefaultThemes.kt`)
- FontManager: font string like `system:FiraCode%20Nerd%20Font` (url-encoded family) or null
- UI scale float or null (system). LanguageManager: language is a locale string or null.
- Custom window: Stock / Acrylic / Native / NativeAcrylic modes, acrylic amount 0..100 (`frameAlpha = 1 - 0.8*amount/100`), native title bar, file `config/window-style.properties`, 300 ms debounced save

### 1.11 Per-download settings
- Speed limit (bytes/s, 0 = none), preferred connection count (null = global), username, password, userAgent, headers, downloadPage (referrer), serverETag, fileChecksum: download item JSON (section 4.2)
- Per-host settings list (`options/perHostSettings.json`, `List<PerHostSettingsItem>`; fields UNVERIFIED)
- Extra settings: power action on finish and force flag (`extra_download_settings/<id>.json`)

### 1.12 Checksum
- `fileChecksum` field on the item (null live). Verification UI and algorithm list: UNVERIFIED.

### 1.13 Other
- KeepAwakeManager (prevents sleep while downloading), SystemUsageMonitor, IApplicationBackgroundTracker (`Di.kt`)
- RemovedDownloadsFromDiskTracker (`trackDeletedFilesOnDisk`)
- FailedDownloads in-memory store and DownloadErrorMapperRegistry (friendly error text)
- CLI `ABDownloadManagerCli`: `download add http|hls`, `pause`, `resume`, `remove`, `show`, `gui run|exit|integration|start-if-not-started`, `nativemessaging install|run|uninstall`. `download add` options: `--link` (required), `--folder`, `--name`, `--download-page`, `--start`, `--start-queue`, `--category`, `--queue`, `--header/-h key=value|key:value`. Prints the new id.
- Crash log `system/log/crash.log`; DownloadMonitor module (`downloader/monitor`, UNVERIFIED)

---

## 2. Download engine behaviour

Defaults (`ENG/DownloadSettings.kt`, `appSettings`): threads 8, dynamicPartCreation true, minPartSize 512 KiB (allowed 1 KiB to 2 GiB), retry count 3 in the app (0 in core), sparse true, useServerLastModifiedTime false, appendExtension false (read at boot only).

- Part splitting: the file is cut into N equal ranges (`SplitToRange.kt`). Live sample: 8 contiguous ranges, each `{from,to,current}`. Parts are created once (`HttpDownloadJob.kt:317-347`) and stored in `parts/<id>.json`.
- Dynamic parts: when a part finishes and free threads remain, the largest remaining part is halved, subject to minPartSize and a 1 MiB safe zone (`PartSplitSupport.kt`).
- Resume: supported only if the probe answered 206 with a Content-Length and a Content-Range full size (`HttpResponseInfo.resumeSupport`). Progress is each part's `current`, autosaved every 1 s (`DownloadJob.kt:55-62`).
- No resume support: engine falls back to a single part and restarts from zero on retry. Exact branch UNVERIFIED (around `HttpDownloadJob.kt:631-693`).
- Retry: a part retries up to PART_MAX_TRIES (10) with a fixed 1000 ms delay (`PartDownloader.kt`). The whole-job retry (`HttpDownloadJob.kt:460-529`) uses the user's retry count. No exponential backoff found.
- Allocation: one pre-sized destination file, all parts write into it through one okio FileHandle at their offsets, so there is NO merge step. With "append extension" on, the name is `<name>.dl-<id>.abdm.part` and is moved to the final name on completion (delete target, then atomic move). Windows long-path trimming applies (`destination/*`).
- Sparse: SPARSE open option on NTFS/ReFS, zero-fill fallback (`SparseFile.desktop.kt`).
- Speed limit: token bucket, waits in 8 KiB units, max chunk 256 KiB (`OkioCustomizedThrottler`); change applies immediately (`bytesPerSecond(changeImmediately=true)`).
- Redirects: OkHttp default follow. Protocols forced to HTTP/1.1 (`Di.kt`, final client). No explicit timeouts (OkHttp 10 s defaults).
- Headers: order is User-Agent (default Chrome 120 string unless set), item headers except Host, `accept-encoding: identity`, Basic auth if username set, item userAgent, Range (`OkHttpHttpDownloaderClient`). Cookies are only whatever the browser put in the item headers.
- Probe: `HttpDownloaderClient.test()` sends `Range: bytes=0-255`, falls back to a plain request. 401 with Basic challenge sets `requireBasicAuth`. A `text/html` reply sets `isWebPage`.
- Server Last-Modified: applied to the finished file only when enabled.
- SSL ignore: trust-all factory plus accept-all hostname verifier when `ignoreSSLCertificates`.
- Proxy: Direct, UseSystem, PAC script, Manual (HTTP or SOCKS) (`OkHttpHttpDownloaderClient`). DNS: system or custom (`AppDns`, `options/dnsSettings.json`).
- Protocols: HTTP/HTTPS and HLS only. No FTP, no torrent, no HTTP/2 (forced off).
- Duplicate detection: `DownloadManager.addDownload` (`DownloadManager.kt:69-143`) checks same destination file and applies OnDuplicateStrategy. Browser double-sends: local 15 s link-only drop.
- Concurrency: queue maxConcurrent plus global max concurrent downloads (0 = unlimited).
- Events: `tryEmit` into a shared flow with buffer 64 (`DownloadManager.kt`).

---

## 3. Integration

### 3.1 Local HTTP API (`INTEG/Integration.kt`, `Endpoints.kt`)
- Server: Ktor CIO, host `127.0.0.1`, port from setting `apiPort` (default 15151). Restarts when settings change. A start failure disables integration (`IntegrationResult` Inactive | Fail | Success(port)).
- Auth: Ktor apiKey plugin. Header name is the Ktor default `X-API-Key` (UNVERIFIED, never set in source; no occurrence found). Compare is plain `==`.
  - `auth-optional`: key checked only when `apiAuthEnabled` is true.
  - `auth-required`: always checked.
- Key handling: `IntegrationSettings(port, apiKeyEnforced, apiKey?)` built in `AppRepository.kt:54-56`; blank key becomes null. Default key is random 24 chars A-Za-z0-9 (`ApiKeyUtil`). Stored plaintext in appSettings.json.

Endpoints:

| Method | Path | Auth | Request JSON | Response |
|---|---|---|---|---|
| POST | /add | optional | AddDownloadsFromIntegration (or legacy bare list of http credentials) | text `OK` |
| GET | /queues | optional | none | `[{"id":Long,"name":String}]` |
| POST | /ping | optional | none | text `pong` |
| POST | /start-headless-download | required | NewDownloadTask | text `OK` (id not returned) |

Models (exact field names):
- AddDownloadsFromIntegration: `items: [credentials]`, `options: {silentAdd:Boolean=false, silentStart:Boolean=false}`
- Credentials polymorphic on `"type"`: `"http"` or `"hls"`; fields `link:String`, `headers:Map<String,String>?=null`, `downloadPage:String?=null`, `suggestedName:String?=null`
- NewDownloadTask: `downloadSource` (credentials), `folder:String?`, `name:String?`, `queueId:Long?`, `categoryId:Long?`, `startDownload:Boolean=false`, `startQueue:Boolean=false`
- Handler logic: `IntegrationHandlerImp.kt:43-105`. `addDownload`: folder defaults to the default download folder, name falls back to suggestedName then last URL segment, `startQueue` (with a queueId) wins over `startDownload`.
- Errors: exceptions bubble to Ktor, HTTP 500.

### 3.2 Native messaging host (`ABDownloadManagerNativeMessagingHost.exe`)
- Framing (`StdioProtocol.kt`): 4-byte length in native byte order (little endian on Windows), then UTF-8 JSON, valid size 1..4 MiB. EOF on the length read ends the process cleanly.
- Envelope: `{"id":String,"content":{"action":String?,"isError":Boolean,"payload":String}}`. `payload` is a JSON string. Browser ids start `B_`, host ids `N_<uuid>`. A reply reuses the request id.
- Actions (`DefinedHandlers.kt`): `add` (payload = AddDownloadsFromIntegration JSON, reply payload `true`), `ping` (reply `true`). Anything else gives an error content `{"errorType":?,"message":?}` with `isError:true`; handler exceptions also come back as errors.
- Launching the app: the host calls `awakeTheApp` (10 s timeout). If the app is not ready it runs `ABDownloadManagerCli gui start-if-not-started`, which polls every 500 ms for up to 10 s and starts `ABDownloadManager.exe --background` detached, then waits for the kRPC service.
- Manifests: name = package name (live `com.abdownloadmanager`). Chrome: `{name, description, path, type:"stdio", allowed_origins:[chromeExtensionOrigin]}`; Firefox: same with `allowed_extensions:[firefoxExtensionId]`. Written under `config/native_messaging/{chrome,firefox}/<pkg>.json`; registry defaults set at `HKCU\SOFTWARE\Google\Chrome\NativeMessagingHosts\<pkg>` and `HKCU\SOFTWARE\Mozilla\NativeMessagingHosts\<pkg>`. Installed only when running from a real install.

### 3.3 Single instance IPC (kRPC)
- `config/app.lock`: RandomAccessFile + FileChannel.tryLock; shutdown hook releases and deletes it.
- `config/app.port`: plain port number text of a CIO server on `127.0.0.1` port 0 (random). Deleted on stop. Websocket `ws://localhost:<port>/single-instance`, kotlinx-rpc, JSON. NO authentication.
- Services: `ISingleInstanceService.awaitReady`; `IDefaultAppIPCService`: `addDownloadByGui(AddDownloadsFromIntegration)`, `addDownload(NewDownloadTask):Long`, `pauseDownload(ids)`, `resumeDownload(ids)`, `removeDownload(ids, alsoRemoveFile)`, `showDownload(ids):[ShowDownloadIPC]`, `watchDownload(ids):Flow`, `listQueues()`.
- ShowDownloadIPC: `{name, folder, id:Long, percent:Int?, status}`; status in Paused, Error, Downloading, Finished, PreparingFile, Resuming, Retrying.
- The Rust rewrite needs a local control channel with the same verbs. The CLI is its only client besides the native host.

### 3.4 Handoff host `com.local.abdm_handoff` (a local addition, NOT in AB source)
- Manifest `~/.abdm/config/native_messaging/handoff/com.local.abdm_handoff.json`: `path` is a PyInstaller exe from `scripts/abdm-handoff/dist/abdm-handoff.exe`, `type:"stdio"`, `allowed_origins:["chrome-extension://oodniaigpjkfedcencfpbafinanfhgom/"]`. Source: `scripts/abdm-handoff/bridge.py`; Zen/Firefox variant via `abdm-handoff.sys.mjs`.
- Framing: 4-byte little-endian length, max 65536. Request `{"filename":"<path of finished browser download>"}`. Reply `{"ok":true,"id":N}` or `{"ok":false,"restored":bool,"error":"<ExceptionName>"}`. CLI mode `--file <path>`.
- Flow: resolve path, require regular non-symlink file outside WINDIR, ProgramFiles, LOCALAPPDATA, APPDATA; sha256; rename to `.<name>.abdm-source-<token>`; write a receipt JSON; serve the file from a throwaway 127.0.0.1 server with Range/HEAD at `/<token>`; run `ABDownloadManagerCli download add http --link http://127.0.0.1:<port>/<token> --folder <parent> --name <name> --start` (45 s timeout); poll `downloadlist/*.json` every 0.2 s for up to 600 s matching the token; on `Completed` check folder/name and sha256, copy the `:Zone.Identifier` stream, delete the source; on Error/timeout pause via CLI, restore the original, keep partial output as `.<name>.abdm-output-<token>`.
- Rewrite must keep: a CLI-compatible `download add http --link --folder --name --start`, `download pause <id>`, item JSON fields `id`, `status`, `folder`, `name`; better, offer an in-process "adopt existing file" call so the handoff needs no local server and no polling.

---

## 4. On-disk formats

Root: `~/.abdm` (or a portable dir). `config/` and `system/` below. JSON is pretty-printed with 4 spaces, `encodeDefaults=true`, `ignoreUnknownKeys=true` (`Di.kt:386-427`). Class discriminator key is `"type"`. Writes go through a tmp file then move (`TransactionalFileSaver`).

Layout:
```
config/app.lock, app.port, appSettings.json, pageStatesStorage.json, window-style.properties
config/pages/lastSavedLocation.json, selectQueue.json
config/options/proxySettings.json, dnsSettings.json, perHostSettings.json
config/download_db/downloadlist/<id>.json + last_id.txt
config/download_db/parts/<id>.json
config/download_db/queues/<id>.json
config/download_db/categories/categories.json
config/download_db/extra_download_settings/<id>.json
config/download_db/extra_queue_settings/<id>.json
config/native_messaging/{chrome,firefox,handoff}/
system/update/, system/log/crash.log, system/downloadData/
```

### 4.1 appSettings.json (flat object; default in brackets)
Strings: theme ["dark"], defaultDarkTheme ["dark"], defaultLightTheme ["light"], language [null], font [null], defaultDownloadFolder [`<Downloads>\ABDM`], generalNotificationSound [""], errorNotificationSound [""], successNotificationSound [""], apiAuthKey [random 24 chars], userAgent [""].
Numbers: uiScale float? [null], threadCount int [8], maxConcurrentDownloads int [3], maxDownloadRetryCount int [3], speedLimit long bytes/s [262144], minPartSize long [524288], apiPort int [15151].
Booleans: showIconLabels [true], useRelativeDateTime [true], dynamicPartCreation [true], useServerLastModifiedTime [false], appendExtensionToIncompleteDownloads [false], useSparseFileAllocation [true on desktop], useAverageSpeed [true], showDownloadProgressDialog [true], showDownloadCompletionDialog [true], autoStartOnBoot [true], notificationSound [true], apiEnabled [true], apiAuthEnabled [false], trackDeletedFilesOnDisk [false], deletePartialFileOnDownloadCancellation [false], ignoreSSLCertificates [false], useCategoryByDefault [true], blockAddWhenLoading [true], mergeTopBarWithTitleBar [true], useNativeMenuBar [false], useSystemTray [true].
Enums: useSpeedLimit `"Disabled"` plus other SpeedLimitMode values (UNVERIFIED names), sizeUnit and speedUnit `"BinaryBytes" | "DecimalBytes" | "BinaryBits" | "DecimalBits"` (default BinaryBytes).
Sources: `shared/app/src/desktopMain/.../IAppSettingsModel.desktop.kt:16-63`, `DefaultAppSettings.kt:11-53`. The three desktop keys use `catch(default)` so a missing key falls back.
The audited live file differs from defaults in theme, font, appendExtension (true), progress/completion dialogs (false), speed and part values, autoStart (false), folder, apiAuthEnabled (true), deletePartial (true), units (DecimalBytes), useCategoryByDefault (false), mergeTopBar (false). The rewrite should import these.

### 4.2 downloadlist/<id>.json
Key order: `type:"http"` (or hls), `link:String`, `headers:Map<String,String>?`, `username:String?`, `password:String?`, `downloadPage:String?`, `userAgent:String?`, `serverETag:String?`, `id:Long`, `folder:String`, `name:String`, `contentLength:Long`, `dateAdded:Long` (epoch ms), `startTime:Long?`, `completeTime:Long?`, `status:String` (Added, Completed, Paused, Error, etc.; full list UNVERIFIED), `preferredConnectionCount:Int?`, `speedLimit:Long` (0 none), `fileChecksum:String?`. `last_id.txt` holds the highest id as text (live 502, about 540 files).

### 4.3 parts/<id>.json
`{"type":"ranges","list":[{"from":Long,"to":Long,"current":Long}, ...]}`; HLS uses `"type":"mediaSegments"` (fields UNVERIFIED).

### 4.4 queues/<id>.json
`{"id":Long,"name":String,"maxConcurrent":Int,"queueItems":[Long],"scheduledTimes":{"daysOfWeek":["MONDAY",...],"startTime":"HH:mm","endTime":"HH:mm","enabledStartTime":Boolean,"enabledEndTime":Boolean},"stopQueueOnEmpty":Boolean}`. Default queue id 0 "Main"; new ids start above 10.

### 4.5 categories.json
`[{"id":Long,"name":String,"icon":String,"path":String,"usePath":Boolean=true,"acceptedFileTypes":[String],"acceptedUrlPatterns":[String],"items":[Long]}]` (`Category.kt:21-40`). Live file is `[]`, so defaults come from code.

### 4.6 Extra settings
- `extra_download_settings/<id>.json`: `{"id":Long,"powerActionTypeOnFinish":String?,"powerActionUseForceOnFinish":Boolean}`
- `extra_queue_settings/<id>.json`: empty live, fields UNVERIFIED.

### 4.7 Other files
- `pageStatesStorage.json`: `home.downloadListState {sizes, sortBy{name,descending}, visibleCells[], order[]}`, `windowSize{first,second}`, `isMaximized`, `categoriesWidth`, plus per-page window sizes.
- `window-style.properties` (Java Properties): `acrylic`, `acrylicAmount`, `nativeTitleBar`.
- `options/proxySettings.json`, `dnsSettings.json`, `perHostSettings.json`: schemas UNVERIFIED (proxy modes Direct, UseSystem, Manual, Pac; DNS modes System or custom address).

---

## 5. Weaknesses, bugs and heavy parts

Bugs and risks (file:line where known):
1. Autosave writes every download's JSON every 1 s while running: `ENG/downloaditem/DownloadJob.kt:55-62`. Heavy disk churn, torn state on crash.
2. Corrupt or unreadable JSON is swallowed to null, so a download silently disappears: `TransactionalFileSaver`. `getAll` reads every file (about 540) each time.
3. `ContextProvider` is a plain map with no locking: `DownloadManager.kt:386-407`. Events use `tryEmit` with buffer 64, so bursts can drop events.
4. `PartDownloader.kt` loop condition `while (coroutineContext.isActive || !stop)` looks wrong (should be `&&`); `GlobalScope` use; one OS thread per part with blocking IO; 8 KiB read buffer.
5. `ManualDownloadQueue.kt:100-104` ensureBooted looks inverted.
6. `HttpDownloadJob.kt:614` carries "TODO probably bug here".
7. Probe issues a real Range GET and a second plain GET on fallback, so a one-time link may be spent.
8. API key compared with `==` (timing), stored plaintext in `appSettings.json`; `/ping` is POST; failures return 500 with no JSON error; `/start-headless-download` does not return the id although IPC does.
9. Request header sets, including cookies, are persisted in plaintext in `downloadlist/*.json`. Rewrite should store cookies in an encrypted store (Windows DPAPI) and expire them.
10. kRPC websocket on a random port has no auth, and a stale `app.port` survives a crash (`DESK/utils/singleInstance/`). Rewrite: named pipe with a per-user ACL.
11. No explicit network timeouts, no backoff, fixed 1 s retry. Core retry default 0 vs app default 3 is confusing.
12. HTTP/2 disabled (forced HTTP/1.1), no FTP.
13. `RenderApi.kt` (local edit) defaults x64 to DIRECT3D without checking driver support; the 15 s duplicate drop is keyed on link only, in memory only, not applied to headless adds (`IntegrationHandlerImp.kt:29-47`).
14. Resume safety: no ETag or Last-Modified check on resume was verified (field `serverETag` exists, enforcement UNVERIFIED).
15. Handoff bridge: 0.2 s polling over the whole `downloadlist` folder for up to 10 minutes and matching a token as a substring of the whole JSON (`scripts/abdm-handoff/bridge.py`).

Heavy parts (memory and CPU; not measured):
- JVM plus Compose Desktop (Skia), Ktor server, OkHttp, kotlinx-rpc and Koin: large resident memory and slow start. A second JVM per CLI or native-host call (`ABDownloadManagerCli`, native messaging host exe) multiplies this.
- Timers: 1 s autosave per job, 500 ms queue shake, 500 ms start-if-not-started poll.
- Rust target: single process with a tokio runtime, async HTTP (reqwest or hyper), one memory-mapped or `FileExt::write_at` destination, SQLite or one append-only journal instead of one JSON per download, a Windows named pipe for CLI and native host, a tiny native-host exe.

Not read (UNVERIFIED, read before porting): `HomeComponent.kt`, `HomePage.kt`, multi-add components, clipboard watcher, notification and sound playback, tray menu items, updater apply step, HLS and monitor module, `CategoryManager.kt`, per-host settings model, proxy and DNS JSON schemas, `CustomWindow.kt` internals.
