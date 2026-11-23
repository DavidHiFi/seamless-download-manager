# The engine library (`seamless_download_core.dll`)

The Windhawk mod runs the same Rust download engine as the standalone app, loaded as a DLL.
Built by `crates/sdm-ffi`. 64-bit only. Each GitHub release ships it as
`seamless_download_core-<version>.dll` next to the installer.

## C functions

```c
int   sdm_start(const wchar_t* data_dir);  // null = %LOCALAPPDATA%\SeamlessDownloadManager
char* sdm_call(const char* json_utf8);      // one command in, one reply out
void  sdm_free(char* reply);                // free every reply from sdm_call / sdm_last_error
void  sdm_stop(void);                       // pause downloads, close the browser link, let go
char* sdm_last_error(void);                 // {"error":"..."} after sdm_start returned -1
```

- `sdm_start` returns 0 when running (a second call is a no-op), 1 when the standalone app
  holds the data folder and would not close within 10 seconds, -1 on failure.
- The engine and the standalone app share one lock (`Local\SeamlessDownloadManager`), so only
  one runs. `sdm_start` asks a running app to close first (its `/v1/exit`), then takes over.
- `sdm_start` also starts the browser link (the local API the extension talks to), on the
  port in settings.
- Calls are safe from any thread. They return quickly; slow work (probe, checksum, update
  check, import) returns a job number to poll.
- Never unload the DLL (`FreeLibrary`): its threads may still be finishing. Call `sdm_stop`
  and leave it loaded.
- Sounds on finish and fail play from the library itself (at most one every 2 seconds),
  following the settings.

## Replies

Every reply is JSON. Failures are `{"error":"plain sentence"}`. Commands that only act reply
`{"ok":true}`.

## Commands

Each request is an object with `cmd` plus the fields shown.

| cmd | fields | reply |
|---|---|---|
| `version` | | `{"version":"0.1.0"}` |
| `status` | | `api_up`, `api_port`, `api_text`, `data_dir` |
| `poll` | `rev` (optional, last rev seen) | `rev`, `queues_rev`, `settings_rev`, `show`, `exit`, `add_requests` (list of add requests from the browser), `power` (`{queue, action}` or null), and `downloads` (full list) only when `rev` differs |
| `list` | | list of downloads |
| `get` | `id` | one download or null |
| `settings` | | the whole settings object |
| `set_settings` | `settings` (only the fields that change) | ok |
| `queues` | | `{queues:[...], active:[ids]}` |
| `save_queue` | `queue` (object; `id` 0 is Main, a new one gets a new id) | `{id}` |
| `delete_queue` / `start_queue` / `stop_queue` | `id` | ok |
| `move_to_queue` | `ids`, `queue` | ok |
| `categories` | | list |
| `save_category` | `category` | `{id}` |
| `delete_category` | `id` | ok |
| `add` | `request` (add request, below) | `{id}` |
| `batch_preview` | `pattern` (with one `*`), `start`, `end`, `padding` ("auto", "none" or a width) | `{count, first, last}` |
| `add_many` | same as `batch_preview`, plus `request` (template) and `start_queue` | `{ids}` |
| `start` | `id` | ok |
| `resume` / `pause` | `ids` | ok |
| `remove` | `ids`, `delete_file` | ok |
| `pause_all` / `resume_all` / `stop_all` | | ok |
| `set_speed_limit` | `id`, `rate` (bytes a second, 0 = none) | ok |
| `probe` | `url`, `headers` (list of `[name, value]`) | `{job}`; result: `final_url`, `file_name`, `total`, `resumable`, `content_type`, `is_web_page`, `needs_auth` |
| `checksum` | `id` or `path`, `kind` (MD5, SHA-1, SHA-256, SHA-512), `expected` (optional) | `{job}`; result: `sum`, `kind`, `matches` (true, false or null) |
| `check_update` | `current` (optional version) | `{job}`; result: `{release:false}` or `{release, tag, url, newer}` |
| `import_abdm` | `path` | `{job}`; result: `report` |
| `job` | `job` | `{done:false}` or `{done:true, ok, result}` / `{done:true, ok:false, error}` (read once) |
| `power` | `action` (sleep, hibernate, shutdown, restart) | ok; with `SDM_POWER_DRY_RUN` set, writes `power-dry-run.txt` instead |

### Download (as in `list`)

`id, name, folder, url, size, downloaded, status, error, speed, eta_secs, date_added,
date_finished, queue_id, category_id, connections, speed_limit`. `status` is one of
`added, queued, downloading, paused, completed, failed, retrying`. Sizes in bytes; `size`,
`eta_secs`, `error` and `date_finished` are null when unknown. Dates in milliseconds since 1970.

### Add request

`url` (required), `kind` (`"Http"` or `"Hls"` for a stream playlist, default Http), `headers`, `referrer`,
`suggested_name`, `folder`, `name`, `queue_id`, `category_id`, `start`, `start_queue`,
`basic_auth` (`[name, password]`), `user_agent`, `threads`, `speed_limit`. Leave out what
you do not set.

### Polling

Poll about four times a second while the window is open and once a second while hidden.
Pass the last `rev`; the download list only comes back when something changed. When
`queues_rev` or `settings_rev` moved, fetch `queues` or `settings` again.
