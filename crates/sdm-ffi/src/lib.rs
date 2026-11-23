//! The download engine as a library, for the Seamless Download Manager Windhawk mod.
//!
//! Four C functions. `sdm_start` opens the data folder and starts the engine and the browser
//! link. `sdm_call` takes one JSON command and returns one JSON reply, which the caller frees
//! with `sdm_free`. `sdm_stop` pauses the downloads and shuts everything down; `sdm_start` may
//! be called again afterwards. The commands are listed in `docs/engine-library.md`.

use std::ffi::{CStr, CString, c_char};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use sdm_core::checksum::{HashKind, hash_file, normalize};
use sdm_core::manager::{AddRequest, Manager, ManagerEvent};
use sdm_core::model::{Category, PowerAction, Queue};
use sdm_core::util::{BatchPadding, expand_batch};
use sdm_core::Settings;
use serde_json::{Value, json};

const RELEASES: &str = "https://api.github.com/repos/DavidHiFi/seamless-download-manager/releases/latest";
/// The same lock the standalone app takes, so only one of them runs. Tests use their own.
const INSTANCE: &str = if cfg!(test) { "Local\\SeamlessDownloadManagerTest" } else { "Local\\SeamlessDownloadManager" };

/// What the poll hands to the window. Filled by the event listener.
#[derive(Default)]
struct Inbox {
    rev: u64,
    queues_rev: u64,
    settings_rev: u64,
    add_requests: Vec<AddRequest>,
    show: bool,
    exit: bool,
    power: Option<(String, PowerAction)>,
}

enum Job {
    Running,
    Done(Result<Value, String>),
}

struct Core {
    rt: tokio::runtime::Runtime,
    manager: Arc<Manager>,
    api: Option<sdm_api::ApiHandle>,
    api_text: String,
    inbox: Arc<Mutex<Inbox>>,
    jobs: Arc<Mutex<HashMap<u64, Job>>>,
    instance: Instance,
}

struct Instance(isize);

impl Drop for Instance {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::Threading::ReleaseMutex;
        unsafe {
            let h = HANDLE(self.0 as *mut _);
            let _ = ReleaseMutex(h);
            let _ = CloseHandle(h);
        }
    }
}

static CORE: Mutex<Option<Core>> = Mutex::new(None);

/// A caught bug while the lock was held must not lock everyone out, `sdm_stop` included.
fn core_lock() -> std::sync::MutexGuard<'static, Option<Core>> {
    CORE.lock().unwrap_or_else(|p| p.into_inner())
}
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);
static LAST_SOUND: AtomicU64 = AtomicU64::new(0);

fn claim_instance() -> Option<Instance> {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::HSTRING;
    unsafe {
        let handle = CreateMutexW(None, true, &HSTRING::from(INSTANCE)).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = windows::Win32::Foundation::CloseHandle(handle);
            return None;
        }
        Some(Instance(handle.0 as isize))
    }
}

/// The standalone app holds the data folder: ask it to close, then wait for it to let go.
fn take_over_from_app(rt: &tokio::runtime::Runtime, dir: &Path) -> Option<Instance> {
    if let Some(i) = claim_instance() {
        return Some(i);
    }
    if let Ok(settings) = Settings::load(&dir.join("settings.json")) {
        rt.block_on(async {
            let _ = reqwest::Client::new()
                .post(format!("http://127.0.0.1:{}/v1/exit", settings.api_port))
                .header("X-Api-Key", settings.api_key)
                .timeout(Duration::from_secs(3))
                .send()
                .await;
        });
    }
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(250));
        if let Some(i) = claim_instance() {
            return Some(i);
        }
    }
    None
}

fn listen(manager: &Arc<Manager>, inbox: Arc<Mutex<Inbox>>) {
    let mut events = manager.subscribe();
    let manager = manager.clone();
    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            let event = events.recv().await;
            let mut b = match inbox.lock() {
                Ok(b) => b,
                Err(_) => break,
            };
            match event {
                Ok(ManagerEvent::Added(_) | ManagerEvent::Changed(_) | ManagerEvent::Removed(_) | ManagerEvent::Progress { .. }) => b.rev += 1,
                Ok(ManagerEvent::QueuesChanged) => {
                    b.queues_rev += 1;
                    b.rev += 1;
                }
                Ok(ManagerEvent::SettingsChanged) => {
                    b.settings_rev += 1;
                    b.rev += 1;
                }
                Ok(ManagerEvent::AddRequested(req)) => b.add_requests.push(*req),
                Ok(ManagerEvent::ShowRequested) => b.show = true,
                Ok(ManagerEvent::ExitRequested) => b.exit = true,
                Ok(ManagerEvent::Done { ok, .. }) => {
                    drop(b);
                    play_done(&manager.settings(), ok);
                }
                Ok(ManagerEvent::QueueFinished { name, action, .. }) => b.power = Some((name, action)),
                Err(RecvError::Lagged(_)) => {
                    b.rev += 1;
                    b.queues_rev += 1;
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// At most one sound every two seconds, so a big batch is quiet.
fn play_done(settings: &Settings, ok: bool) {
    if !settings.notification_sound {
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let last = LAST_SOUND.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 2000 || LAST_SOUND.compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed).is_err() {
        return;
    }
    play(if ok { &settings.success_notification_sound } else { &settings.error_notification_sound }, ok);
}

fn play(file: &str, ok: bool) {
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
    use windows::core::HSTRING;
    let file = file.trim();
    unsafe {
        if !file.is_empty() && Path::new(file).is_file() {
            let _ = PlaySoundW(&HSTRING::from(file), None, SND_FILENAME | SND_ASYNC | SND_NODEFAULT);
        } else {
            let alias = if ok { "SystemAsterisk" } else { "SystemHand" };
            let _ = PlaySoundW(&HSTRING::from(alias), None, SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
        }
    }
}

fn run_power(dir: &Path, action: PowerAction) -> Result<()> {
    if std::env::var_os("SDM_POWER_DRY_RUN").is_some() {
        std::fs::write(dir.join("power-dry-run.txt"), format!("{}\n", action.as_str()))?;
        return Ok(());
    }
    match action {
        PowerAction::None => Ok(()),
        PowerAction::Sleep | PowerAction::Hibernate => {
            let ok = unsafe { windows::Win32::System::Power::SetSuspendState(action == PowerAction::Hibernate, false, false) };
            if ok { Ok(()) } else { bail!("Windows said no") }
        }
        PowerAction::Shutdown | PowerAction::Restart => {
            use std::os::windows::process::CommandExt;
            let flag = if action == PowerAction::Shutdown { "/s" } else { "/r" };
            std::process::Command::new("shutdown").args([flag, "/t", "0"]).creation_flags(0x0800_0000).spawn()?;
            Ok(())
        }
    }
}

pub fn is_newer(tag: &str, current: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.trim().trim_start_matches(['v', 'V']).split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).take(3).map(|p| p.parse().unwrap_or(0)).collect()
    }
    let (a, b) = (parts(tag), parts(current));
    if a.is_empty() {
        return false;
    }
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

async fn latest_release(current: &str) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .user_agent("SeamlessDownloadManager")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| if e.is_timeout() { "GitHub did not answer in time".to_string() } else { "GitHub could not be reached".to_string() })?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(json!({ "release": false }));
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub answered {}", resp.status().as_u16()));
    }
    let v: Value = resp.json().await.map_err(|_| "GitHub sent something unexpected".to_string())?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    let url = v.get("html_url").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    Ok(json!({ "release": true, "tag": tag, "url": url, "newer": is_newer(&tag, current) }))
}

fn arg<'a>(req: &'a Value, key: &str) -> Result<&'a Value> {
    req.get(key).ok_or_else(|| anyhow!("missing \"{key}\""))
}

fn id(req: &Value) -> Result<i64> {
    arg(req, "id")?.as_i64().ok_or_else(|| anyhow!("\"id\" must be a number"))
}

fn ids(req: &Value) -> Result<Vec<i64>> {
    arg(req, "ids")?.as_array().ok_or_else(|| anyhow!("\"ids\" must be a list"))?.iter().map(|v| v.as_i64().ok_or_else(|| anyhow!("ids must be numbers"))).collect()
}

fn text<'a>(req: &'a Value, key: &str) -> Result<&'a str> {
    arg(req, key)?.as_str().ok_or_else(|| anyhow!("\"{key}\" must be text"))
}

fn padding(req: &Value) -> BatchPadding {
    match req.get("padding") {
        Some(Value::Number(n)) => BatchPadding::Width(n.as_u64().unwrap_or(0).min(20) as usize),
        Some(Value::String(s)) if s == "none" => BatchPadding::None,
        _ => BatchPadding::Auto,
    }
}

fn hash_kind(name: &str) -> Result<HashKind> {
    HashKind::ALL.into_iter().find(|k| k.label().eq_ignore_ascii_case(name) || k.label().replace('-', "").eq_ignore_ascii_case(name)).ok_or_else(|| anyhow!("unknown check type {name}"))
}

/// Merges the given fields into the current settings, so the caller may send only what changed.
fn merged_settings(current: &Settings, patch: &Value) -> Result<Settings> {
    let mut base = serde_json::to_value(current)?;
    let (Some(base_map), Some(patch_map)) = (base.as_object_mut(), patch.as_object()) else {
        bail!("\"settings\" must be an object");
    };
    for (k, v) in patch_map {
        base_map.insert(k.clone(), v.clone());
    }
    serde_json::from_value(base).context("a setting has the wrong type")
}

fn spawn_job<F>(core: &Core, fut: F) -> Value
where
    F: std::future::Future<Output = Result<Value, String>> + Send + 'static,
{
    let job = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
    core.jobs.lock().unwrap().insert(job, Job::Running);
    let jobs = core.jobs.clone();
    core.rt.spawn(async move {
        let out = fut.await;
        jobs.lock().unwrap().insert(job, Job::Done(out));
    });
    json!({ "job": job })
}

fn command(core: &Core, req: &Value) -> Result<Value> {
    let _enter = core.rt.enter();
    let m = &core.manager;
    let cmd = text(req, "cmd")?;
    Ok(match cmd {
        "version" => json!({ "version": env!("CARGO_PKG_VERSION") }),
        "status" => json!({
            "api_up": core.api.is_some(),
            "api_port": core.api.as_ref().map(|a| a.port()).unwrap_or(0),
            "api_text": core.api_text,
            "data_dir": m.data_dir().to_string_lossy(),
        }),
        "poll" => {
            let since = req.get("rev").and_then(Value::as_u64);
            let mut b = core.inbox.lock().unwrap();
            let mut out = json!({
                "rev": b.rev,
                "queues_rev": b.queues_rev,
                "settings_rev": b.settings_rev,
                "show": std::mem::take(&mut b.show),
                "exit": std::mem::take(&mut b.exit),
                "add_requests": std::mem::take(&mut b.add_requests),
                "power": b.power.take().map(|(queue, action)| json!({ "queue": queue, "action": action.as_str() })),
            });
            if since != Some(b.rev) {
                out["downloads"] = serde_json::to_value(m.list())?;
            }
            out
        }
        "list" => serde_json::to_value(m.list())?,
        "get" => serde_json::to_value(m.get(id(req)?))?,
        "settings" => serde_json::to_value(m.settings())?,
        "set_settings" => {
            let next = merged_settings(&m.settings(), arg(req, "settings")?)?;
            m.update_settings(next)?;
            json!({ "ok": true })
        }
        "queues" => json!({ "queues": m.queues(), "active": m.active_queue_ids() }),
        "save_queue" => json!({ "id": m.save_queue(serde_json::from_value::<Queue>(arg(req, "queue")?.clone())?)? }),
        "delete_queue" => {
            m.delete_queue(id(req)?)?;
            json!({ "ok": true })
        }
        "start_queue" => {
            m.start_queue(id(req)?);
            json!({ "ok": true })
        }
        "stop_queue" => {
            m.stop_queue(id(req)?);
            json!({ "ok": true })
        }
        "move_to_queue" => {
            m.move_to_queue(&ids(req)?, arg(req, "queue")?.as_i64().ok_or_else(|| anyhow!("\"queue\" must be a number"))?)?;
            json!({ "ok": true })
        }
        "categories" => serde_json::to_value(m.categories())?,
        "save_category" => json!({ "id": m.save_category(serde_json::from_value::<Category>(arg(req, "category")?.clone())?)? }),
        "delete_category" => {
            m.delete_category(id(req)?)?;
            json!({ "ok": true })
        }
        "add" => {
            let add: AddRequest = serde_json::from_value(arg(req, "request")?.clone())?;
            json!({ "id": m.add(add)? })
        }
        "batch_preview" => {
            let links = expand_batch(text(req, "pattern")?, arg(req, "start")?.as_u64().unwrap_or(0), arg(req, "end")?.as_u64().unwrap_or(0), padding(req)).map_err(|e| anyhow!(e))?;
            json!({ "count": links.len(), "first": links.first(), "last": links.last() })
        }
        "add_many" => {
            let links = expand_batch(text(req, "pattern")?, arg(req, "start")?.as_u64().unwrap_or(0), arg(req, "end")?.as_u64().unwrap_or(0), padding(req)).map_err(|e| anyhow!(e))?;
            let template: AddRequest = match req.get("request") {
                Some(v) => serde_json::from_value(v.clone())?,
                None => AddRequest::default(),
            };
            let mut added = Vec::new();
            for link in links {
                let mut one = template.clone();
                one.url = link;
                one.name = None;
                added.push(m.add(one)?);
            }
            if req.get("start_queue").and_then(Value::as_bool).unwrap_or(false) {
                m.start_queue(template.queue_id.unwrap_or(0));
            }
            json!({ "ids": added })
        }
        "start" => {
            m.start(id(req)?);
            json!({ "ok": true })
        }
        "resume" => {
            for i in ids(req)? {
                m.resume(i);
            }
            json!({ "ok": true })
        }
        "pause" => {
            for i in ids(req)? {
                m.pause(i);
            }
            json!({ "ok": true })
        }
        "remove" => {
            let delete = req.get("delete_file").and_then(Value::as_bool).unwrap_or(false);
            for i in ids(req)? {
                m.remove(i, delete)?;
            }
            json!({ "ok": true })
        }
        "pause_all" => {
            m.pause_all();
            json!({ "ok": true })
        }
        "resume_all" => {
            m.resume_all();
            json!({ "ok": true })
        }
        "stop_all" => {
            m.stop_all();
            json!({ "ok": true })
        }
        "set_speed_limit" => {
            m.set_speed_limit(id(req)?, arg(req, "rate")?.as_u64().unwrap_or(0));
            json!({ "ok": true })
        }
        "probe" => {
            let url = text(req, "url")?.to_string();
            let headers: Vec<(String, String)> = match req.get("headers") {
                Some(v) => serde_json::from_value(v.clone())?,
                None => Vec::new(),
            };
            let m = m.clone();
            spawn_job(core, async move {
                let info = m.probe(&url, headers).await.map_err(|e| e.to_string())?;
                serde_json::to_value(info).map_err(|e| e.to_string())
            })
        }
        "checksum" => {
            let path = match req.get("path").and_then(Value::as_str) {
                Some(p) => PathBuf::from(p),
                None => {
                    let d = m.get(id(req)?).ok_or_else(|| anyhow!("no download with that id"))?;
                    Path::new(&d.folder).join(&d.name)
                }
            };
            let kind = hash_kind(req.get("kind").and_then(Value::as_str).unwrap_or("SHA-256"))?;
            let expected = req.get("expected").and_then(Value::as_str).map(normalize).unwrap_or_default();
            spawn_job(core, async move {
                let sum = tokio::task::spawn_blocking(move || hash_file(kind, &path)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
                let matches = if expected.is_empty() { Value::Null } else { Value::Bool(expected == sum) };
                Ok(json!({ "sum": sum, "kind": kind.label(), "matches": matches }))
            })
        }
        "check_update" => {
            let current = req.get("current").and_then(Value::as_str).unwrap_or(env!("CARGO_PKG_VERSION")).to_string();
            spawn_job(core, async move { latest_release(&current).await })
        }
        "job" => {
            let job = arg(req, "job")?.as_u64().ok_or_else(|| anyhow!("\"job\" must be a number"))?;
            let mut jobs = core.jobs.lock().unwrap();
            match jobs.get(&job) {
                None => bail!("no job {job}"),
                Some(Job::Running) => json!({ "done": false }),
                Some(Job::Done(_)) => match jobs.remove(&job) {
                    Some(Job::Done(Ok(v))) => json!({ "done": true, "ok": true, "result": v }),
                    Some(Job::Done(Err(e))) => json!({ "done": true, "ok": false, "error": e }),
                    _ => unreachable!(),
                },
            }
        }
        "power" => {
            let action = PowerAction::parse(text(req, "action")?);
            run_power(m.data_dir(), action)?;
            json!({ "ok": true })
        }
        "import_abdm" => {
            let path = PathBuf::from(text(req, "path")?);
            let m = m.clone();
            spawn_job(core, async move {
                let report = tokio::task::spawn_blocking(move || m.import_abdm(&path)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
                Ok(json!({ "report": format!("{report:?}") }))
            })
        }
        other => bail!("unknown command {other}"),
    })
}

static CRASH_FILE: OnceLock<PathBuf> = OnceLock::new();

/// Writes every engine bug to `engine-crash.txt` in the data folder (time, thread, place,
/// message), so a failure inside Windhawk's process leaves its reason behind.
fn note_panics(dir: &Path) {
    if CRASH_FILE.set(dir.join("engine-crash.txt")).is_err() {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = CRASH_FILE.get() {
            let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
            let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
            let what = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_default();
            let line = format!("{when} {} thread={thread} at={place} {what}\n", env!("CARGO_PKG_VERSION"));
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                use std::io::Write;
                let _ = f.write_all(line.as_bytes());
            }
        }
        previous(info);
    }));
}

fn start(dir: PathBuf) -> Result<i32> {
    note_panics(&dir);
    let mut slot = core_lock();
    if slot.is_some() {
        return Ok(0);
    }
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).thread_name("sdm-core").enable_all().build()?;
    let Some(instance) = take_over_from_app(&rt, &dir) else {
        return Ok(1);
    };
    let _enter = rt.enter();
    let manager = Manager::open(dir)?;
    let inbox = Arc::new(Mutex::new(Inbox { rev: 1, queues_rev: 1, settings_rev: 1, ..Default::default() }));
    listen(&manager, inbox.clone());
    let (api, api_text) = match rt.block_on(sdm_api::serve(manager.clone())) {
        Ok(h) => {
            let t = format!("Listening on port {}.", h.port());
            (Some(h), t)
        }
        Err(e) => (None, format!("Could not start: {e}")),
    };
    drop(_enter);
    *slot = Some(Core { rt, manager, api, api_text, inbox, jobs: Arc::new(Mutex::new(HashMap::new())), instance });
    Ok(0)
}

fn stop() {
    let Some(core) = core_lock().take() else { return };
    let Core { rt, manager, api, instance, .. } = core;
    {
        let _enter = rt.enter();
        manager.pause_all();
    }
    if let Some(api) = api {
        rt.block_on(api.stop());
    }
    // The caller is a plain thread, so the sleep is built inside the runtime: tokio's
    // timer lookup panics when `sleep` is created outside it (the 0.1.0 switch-off crash).
    rt.block_on(async { tokio::time::sleep(Duration::from_millis(300)).await });
    drop(manager);
    rt.shutdown_timeout(Duration::from_secs(3));
    drop(instance);
}

fn reply(v: Value) -> *mut c_char {
    CString::new(v.to_string()).unwrap_or_default().into_raw()
}

/// Opens `data_dir` (UTF-16, zero-terminated; null means `%LOCALAPPDATA%\SeamlessDownloadManager`)
/// and starts the engine and the browser link. 0 = running, 1 = the standalone app would not
/// let go of the data folder, -1 = failed (the reason is in `sdm_last_error`).
///
/// # Safety
/// `data_dir` is null or points to a zero-terminated UTF-16 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sdm_start(data_dir: *const u16) -> i32 {
    let dir = if data_dir.is_null() {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join("SeamlessDownloadManager")
    } else {
        let mut len = 0;
        while unsafe { *data_dir.add(len) } != 0 {
            len += 1;
        }
        PathBuf::from(String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(data_dir, len) }))
    };
    match std::panic::catch_unwind(|| start(dir)) {
        Ok(Ok(code)) => code,
        Ok(Err(e)) => {
            *LAST_ERROR.lock().unwrap_or_else(|p| p.into_inner()) = e.to_string();
            -1
        }
        Err(_) => -1,
    }
}

static LAST_ERROR: Mutex<String> = Mutex::new(String::new());

/// The reason the last `sdm_start` failed, as UTF-8 JSON text. Free with `sdm_free`.
#[unsafe(no_mangle)]
pub extern "C" fn sdm_last_error() -> *mut c_char {
    reply(json!({ "error": LAST_ERROR.lock().map(|e| e.clone()).unwrap_or_default() }))
}

/// One JSON command in, one JSON reply out (`{"ok":true,...}` or `{"error":"..."}` for
/// failures). Both UTF-8. Free the reply with `sdm_free`.
///
/// # Safety
/// `request` is null or a zero-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sdm_call(request: *const c_char) -> *mut c_char {
    if request.is_null() {
        return reply(json!({ "error": "empty request" }));
    }
    let text = unsafe { CStr::from_ptr(request) }.to_string_lossy().into_owned();
    let out = std::panic::catch_unwind(|| -> Result<Value> {
        let req: Value = serde_json::from_str(&text).context("the request is not JSON")?;
        let guard = core_lock();
        let core = guard.as_ref().ok_or_else(|| anyhow!("the engine is not running"))?;
        command(core, &req)
    });
    match out {
        Ok(Ok(v)) => reply(v),
        Ok(Err(e)) => reply(json!({ "error": e.to_string() })),
        Err(_) => reply(json!({ "error": "the engine hit a bug" })),
    }
}

/// Frees a reply from `sdm_call` or `sdm_last_error`.
///
/// # Safety
/// `p` came from this library and was not freed yet, or is null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sdm_free(p: *mut c_char) {
    if !p.is_null() {
        drop(unsafe { CString::from_raw(p) });
    }
}

/// Pauses the downloads (they resume on the next start), stops the browser link and the
/// engine, and lets go of the data folder. Safe to call twice.
#[unsafe(no_mangle)]
pub extern "C" fn sdm_stop() {
    let _ = std::panic::catch_unwind(stop);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(v: Value) -> Value {
        let c = CString::new(v.to_string()).unwrap();
        let p = unsafe { sdm_call(c.as_ptr()) };
        let out: Value = serde_json::from_str(&unsafe { CStr::from_ptr(p) }.to_string_lossy()).unwrap();
        unsafe { sdm_free(p) };
        out
    }

    #[test]
    fn start_call_stop() {
        let dir = tempfile::tempdir().unwrap();
        let wide: Vec<u16> = dir.path().as_os_str().to_string_lossy().encode_utf16().chain([0]).collect();
        // Port 0 so the test never meets a real copy on the default port.
        std::fs::write(dir.path().join("settings.json"), r#"{"api_port":0}"#).ok();
        assert_eq!(unsafe { sdm_start(wide.as_ptr()) }, 0);
        assert_eq!(unsafe { sdm_start(wide.as_ptr()) }, 0, "a second start is a no-op");
        assert_eq!(call(json!({"cmd":"version"}))["version"], env!("CARGO_PKG_VERSION"));
        let poll = call(json!({"cmd":"poll"}));
        assert!(poll["downloads"].is_array());
        let rev = poll["rev"].clone();
        assert!(call(json!({"cmd":"poll","rev":rev}))["downloads"].is_null());
        let set = call(json!({"cmd":"set_settings","settings":{"notification_sound":false}}));
        assert_eq!(set["ok"], true, "{set}");
        assert_eq!(call(json!({"cmd":"settings"}))["notification_sound"], false);
        let preview = call(json!({"cmd":"batch_preview","pattern":"https://e.test/f*.bin","start":1,"end":12}));
        assert_eq!(preview["count"], 12);
        assert_eq!(preview["first"], "https://e.test/f01.bin");
        assert!(call(json!({"cmd":"nope"}))["error"].is_string());
        assert!(call(json!({"cmd":"queues"}))["queues"].is_array());
        sdm_stop();
        assert!(call(json!({"cmd":"version"}))["error"].is_string());
        sdm_stop();
        // sdm_stop swallows a panic, so check the crash note: a caught bug must still fail here.
        let crash = std::fs::read_to_string(dir.path().join("engine-crash.txt")).unwrap_or_default();
        assert!(crash.is_empty(), "the engine hit a bug: {crash}");
    }

    #[test]
    fn versions_compare() {
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
    }
}
