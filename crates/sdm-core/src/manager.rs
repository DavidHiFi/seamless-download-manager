use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, mpsc as std_mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Datelike, Local, NaiveDateTime, NaiveTime};
use sdm_engine::{Engine, EngineConfig, Event, JobHandle, JobSpec, OnExists, SpeedLimiter};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use windows::Win32::System::Power::{ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState};

use crate::model::{
    Category, DownloadItem, MAIN_QUEUE_ID, PowerAction, Queue, ResumeState, SourceKind, Status, Weekday,
    default_categories,
};
use crate::settings::Settings;
use crate::store::Store;
use crate::util::{name_from_url, now_ms, sanitize_name};

pub use sdm_engine::ProbeInfo;

const DUPLICATE_WINDOW: Duration = Duration::from_secs(15);
const SAVE_EVERY: Duration = Duration::from_secs(2);

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AddRequest {
    pub url: String,
    pub kind: SourceKind,
    pub headers: Vec<(String, String)>,
    pub referrer: Option<String>,
    pub suggested_name: Option<String>,
    pub folder: Option<String>,
    pub name: Option<String>,
    pub queue_id: Option<i64>,
    pub category_id: Option<i64>,
    pub start: bool,
    pub start_queue: bool,
    /// Sign-in name and password for this link.
    #[serde(default)]
    pub basic_auth: Option<(String, String)>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub threads: Option<u32>,
    /// Bytes per second, 0 = none.
    #[serde(default)]
    pub speed_limit: u64,
    /// Download it even when the same link already finished and its file is still there.
    #[serde(default)]
    pub again: bool,
}

impl fmt::Debug for AddRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddRequest")
            .field("url", &self.url)
            .field("headers", &self.headers.len())
            .field("start", &self.start)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DownloadView {
    pub id: i64,
    pub name: String,
    pub folder: String,
    pub url: String,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub status: Status,
    pub error: Option<String>,
    pub speed: u64,
    pub eta_secs: Option<u64>,
    pub date_added: i64,
    pub date_finished: Option<i64>,
    pub queue_id: Option<i64>,
    pub category_id: Option<i64>,
    pub connections: u32,
    /// This download's own limit in bytes per second, 0 when it has none.
    pub speed_limit: u64,
}

#[derive(Clone, Debug)]
pub enum ManagerEvent {
    Added(i64),
    Changed(i64),
    Removed(i64),
    Progress { id: i64, downloaded: u64, total: Option<u64>, speed: u64 },
    SettingsChanged,
    QueuesChanged,
    /// The browser asked for the add dialog (`silentAdd` false).
    AddRequested(Box<AddRequest>),
    /// A second launch asked the running window to come forward.
    ShowRequested,
    /// The command line asked the app to close.
    ExitRequested,
    /// A download finished (true) or failed (false), for sounds.
    Done { id: i64, ok: bool },
    /// A started queue ran out of work with nothing failed, and has a power action.
    QueueFinished { id: i64, name: String, action: PowerAction },
    /// Something the person should see in a pop-up: a finished, failed, skipped, unpacked or
    /// flagged download. `path` is the file or folder the pop-up's buttons open.
    Notice(Notice),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Notice {
    pub id: i64,
    /// done, unpacked, failed, already, threat or nospace.
    pub kind: &'static str,
    pub name: String,
    pub path: String,
    pub text: String,
}

const MB: u64 = 1024 * 1024;
const DAY_MS: i64 = 86_400_000;
const CLEANUP_EVERY: Duration = Duration::from_secs(3600);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// What gets a Windows Security scan when it finishes.
const SCAN_EXTS: &[&str] = &[
    "exe", "msi", "msix", "msixbundle", "appx", "appxbundle", "bat", "cmd", "ps1", "vbs", "js", "jse", "wsf", "scr",
    "com", "dll", "jar", "iso", "img", "zip", "7z", "rar", "tar", "gz", "tgz", "xz", "bz2", "zst", "cab",
];
/// What Windows' own tar can unpack.
const ARCHIVE_EXTS: &[&str] = &["zip", "7z", "rar", "tar", "tgz", "tbz2", "txz", "gz", "bz2", "xz", "zst"];

fn ext_of(name: &str) -> String {
    Path::new(name).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

pub(crate) fn is_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ARCHIVE_EXTS.contains(&ext_of(&lower).as_str())
        // A lone .gz or .xz is one compressed file, not an archive, unless it is a tarball.
        && (!matches!(ext_of(&lower).as_str(), "gz" | "bz2" | "xz" | "zst") || lower.contains(".tar."))
}

pub(crate) fn wants_scan(name: &str) -> bool {
    SCAN_EXTS.contains(&ext_of(name).as_str())
}

/// The folder an archive unpacks into: its name without the archive ending, made unique.
pub(crate) fn unpack_folder(archive: &Path) -> PathBuf {
    let name = archive.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "unpacked".into());
    let lower = name.to_ascii_lowercase();
    let cut = [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"]
        .iter()
        .find(|e| lower.ends_with(*e))
        .map(|e| name.len() - e.len())
        .or_else(|| name.rfind('.'))
        .filter(|&i| i > 0)
        .unwrap_or(name.len());
    let stem = name[..cut].trim_end_matches(['.', ' ']).to_string();
    let parent = archive.parent().unwrap_or(Path::new("."));
    let mut dest = parent.join(&stem);
    let mut n = 2;
    while dest.exists() {
        dest = parent.join(format!("{stem} ({n})"));
        n += 1;
    }
    dest
}

fn free_bytes(folder: &str) -> Option<u64> {
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::core::HSTRING;
    // The folder may not exist yet; its drive does.
    let mut probe = Path::new(folder);
    while !probe.exists() {
        probe = probe.parent()?;
    }
    let mut free = 0u64;
    unsafe { GetDiskFreeSpaceExW(&HSTRING::from(probe.as_os_str()), Some(&mut free), None, None).ok()? };
    Some(free)
}

pub(crate) fn human_size(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 { format!("{gb:.1} GB") } else { format!("{} MB", bytes / MB) }
}

fn hidden(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    cmd
}

/// True when Windows Security says the file holds a threat. Any other outcome (no scanner,
/// scanner off, timeout) counts as clean, because the file was already checked on write.
fn defender_flags(path: &Path) -> bool {
    let scanner = Path::new(r"C:\Program Files\Windows Defender\MpCmdRun.exe");
    if !scanner.is_file() {
        return false;
    }
    let Ok(mut child) = hidden(&scanner.to_string_lossy()).args(["-Scan", "-ScanType", "3", "-File"]).arg(path).spawn() else {
        return false;
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code() == Some(2),
            Ok(None) if started.elapsed() < Duration::from_secs(180) => std::thread::sleep(Duration::from_millis(200)),
            _ => {
                let _ = child.kill();
                return false;
            }
        }
    }
}

fn unpack(archive: &Path) -> Result<PathBuf> {
    let dest = unpack_folder(archive);
    std::fs::create_dir_all(&dest).context("could not make the folder to unpack into")?;
    let tar = Path::new(r"C:\Windows\System32\tar.exe");
    let ok = hidden(&tar.to_string_lossy()).arg("-xf").arg(archive).arg("-C").arg(&dest).status().is_ok_and(|s| s.success());
    if !ok {
        let _ = std::fs::remove_dir_all(&dest);
        bail!("Windows could not unpack it");
    }
    Ok(dest)
}

/// Sends a file to the Recycle Bin, so turning on "delete the archive" never loses data.
fn recycle(path: &Path) -> bool {
    use windows::Win32::UI::Shell::{FO_DELETE, FOF_ALLOWUNDO, FOF_NO_UI, SHFILEOPSTRUCTW, SHFileOperationW};
    use std::os::windows::ffi::OsStrExt;
    // SHFileOperation takes a list that ends in two nulls.
    let mut from: Vec<u16> = path.as_os_str().encode_wide().chain([0, 0]).collect();
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: windows::core::PCWSTR(from.as_mut_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NO_UI).0 as u16,
        ..Default::default()
    };
    unsafe { SHFileOperationW(&mut op) == 0 && !op.fAnyOperationsAborted.as_bool() }
}

enum RunCmd {
    Pause,
    SetSpeedLimit(u64),
}

struct Running {
    cmd: mpsc::UnboundedSender<RunCmd>,
    speed: u64,
    connections: u32,
    join: Option<JoinHandle<Option<ResumeState>>>,
}

struct Recent {
    key: (String, String),
    at: Instant,
    id: i64,
}

struct State {
    items: BTreeMap<i64, DownloadItem>,
    queues: Vec<Queue>,
    categories: Vec<Category>,
    settings: Settings,
    running: HashMap<i64, Running>,
    manual: HashSet<i64>,
    active_queues: HashSet<i64>,
    recent: Vec<Recent>,
    engine: Option<Engine>,
    last_tick: Option<NaiveDateTime>,
    /// Downloads being stopped because their drive is too full, with the reason to show.
    no_space: HashMap<i64, String>,
    last_cleanup: Option<Instant>,
}

struct KeepAwake {
    tx: std_mpsc::Sender<bool>,
    on: AtomicBool,
}

impl KeepAwake {
    fn new() -> KeepAwake {
        let (tx, rx) = std_mpsc::channel::<bool>();
        std::thread::Builder::new()
            .name("sdm-keep-awake".into())
            .spawn(move || {
                for on in rx {
                    let flags = if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
                    unsafe {
                        SetThreadExecutionState(flags);
                    }
                }
            })
            .ok();
        KeepAwake { tx, on: AtomicBool::new(false) }
    }

    fn set(&self, on: bool) {
        if self.on.swap(on, Ordering::SeqCst) != on {
            let _ = self.tx.send(on);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScheduleAction {
    Start,
    Stop,
}

fn parse_hhmm(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M").ok()
}

/// Queue schedule moments that fall in the window (`last`, `now`].
pub(crate) fn schedule_actions(queues: &[Queue], last: NaiveDateTime, now: NaiveDateTime) -> Vec<(i64, ScheduleAction)> {
    let today = Weekday::from_chrono(now.date().weekday());
    let mut actions = Vec::new();
    for queue in queues {
        if !queue.schedule.days.contains(&today) {
            continue;
        }
        let due = |text: &Option<String>| {
            text.as_deref()
                .and_then(parse_hhmm)
                .map(|t| now.date().and_time(t))
                .is_some_and(|moment| moment > last && moment <= now)
        };
        if queue.schedule.enabled_start && due(&queue.schedule.start) {
            actions.push((queue.id, ScheduleAction::Start));
        }
        if queue.schedule.enabled_end && due(&queue.schedule.end) {
            actions.push((queue.id, ScheduleAction::Stop));
        }
    }
    actions
}

pub struct Manager {
    data_dir: PathBuf,
    store: Store,
    state: Mutex<State>,
    events: broadcast::Sender<ManagerEvent>,
    rt: tokio::runtime::Handle,
    limiter: Arc<SpeedLimiter>,
    awake: KeepAwake,
}

impl Manager {
    /// Must be called inside a tokio runtime.
    pub fn open(data_dir: PathBuf) -> Result<Arc<Manager>> {
        let rt = tokio::runtime::Handle::try_current().context("Manager::open must run inside a tokio runtime")?;
        std::fs::create_dir_all(&data_dir)?;
        let settings = Settings::load(&data_dir.join("settings.json"))?;
        let store = Store::open(&data_dir.join("sdm.db"))?;

        if store.load_categories()?.is_empty() {
            for category in default_categories(&settings.default_folder()) {
                store.save_category(&category)?;
            }
        }
        if !store.load_queues()?.iter().any(|q| q.id == MAIN_QUEUE_ID) {
            store.save_queue(&Queue::main())?;
        }

        let categories = store.load_categories()?;
        let mut items = BTreeMap::new();
        for mut item in store.load_downloads()? {
            let mut changed = false;
            if matches!(item.status, Status::Downloading | Status::Retrying | Status::Queued) {
                item.status = Status::Paused;
                changed = true;
            }
            // Imports from AB only carry categories AB filed the item under, so fill the gaps
            // with the same rule new downloads get.
            if item.category_id.is_none() {
                item.category_id = categories.iter().find(|c| c.matches(&item.name, &item.url)).map(|c| c.id);
                changed |= item.category_id.is_some();
            }
            if changed {
                store.update_download(&item)?;
            }
            items.insert(item.id, item);
        }

        let (events, _) = broadcast::channel(1024);
        let limiter = Arc::new(SpeedLimiter::new(settings.effective_speed_limit()));
        let state = State {
            items,
            queues: store.load_queues()?,
            categories,
            settings,
            running: HashMap::new(),
            manual: HashSet::new(),
            active_queues: HashSet::new(),
            recent: Vec::new(),
            engine: None,
            last_tick: None,
            no_space: HashMap::new(),
            last_cleanup: None,
        };
        let manager = Arc::new(Manager { data_dir, store, state: Mutex::new(state), events, rt, limiter, awake: KeepAwake::new() });
        Self::spawn_ticker(&manager);
        Ok(manager)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ManagerEvent> {
        self.events.subscribe()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit(&self, event: ManagerEvent) {
        let _ = self.events.send(event);
    }

    fn persist(&self, item: &DownloadItem) {
        if let Err(e) = self.store.update_download(item) {
            tracing::warn!("could not save download {}: {e}", item.id);
        }
    }

    fn persist_queue(&self, queue: &Queue) {
        if let Err(e) = self.store.save_queue(queue) {
            tracing::warn!("could not save queue {}: {e}", queue.id);
        }
    }

    fn spawn_ticker(this: &Arc<Manager>) {
        let weak = Arc::downgrade(this);
        this.rt.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let Some(manager) = weak.upgrade() else { break };
                manager.schedule_tick(Local::now().naive_local());
                manager.clear_old_finished(now_ms(), false);
            }
        });
    }

    /// Takes finished entries older than the setting off the list. Their files stay. Runs at
    /// most once an hour unless `force`. Returns how many went.
    pub fn clear_old_finished(self: &Arc<Self>, now: i64, force: bool) -> usize {
        let doomed: Vec<i64> = {
            let mut st = self.lock();
            if !force && st.last_cleanup.is_some_and(|at| at.elapsed() < CLEANUP_EVERY) {
                return 0;
            }
            st.last_cleanup = Some(Instant::now());
            let days = st.settings.clear_finished_after_days;
            if days == 0 {
                return 0;
            }
            let cutoff = now - days as i64 * DAY_MS;
            st.items
                .values()
                .filter(|i| i.status == Status::Completed && i.date_finished.is_some_and(|d| d < cutoff))
                .map(|i| i.id)
                .collect()
        };
        doomed.iter().filter(|id| self.remove(**id, false).is_ok()).count()
    }

    fn notice(&self, id: i64, kind: &'static str, name: &str, path: &Path, text: impl Into<String>) {
        self.emit(ManagerEvent::Notice(Notice {
            id,
            kind,
            name: name.to_string(),
            path: path.to_string_lossy().into_owned(),
            text: text.into(),
        }));
    }

    /// Scans and unpacks a finished file off the async threads, then shows one pop-up.
    fn after_finish(self: &Arc<Self>, id: i64, path: PathBuf) {
        let s = self.lock().settings.clone();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let scan = s.scan_downloads && wants_scan(&name);
        let extract = s.extract_archives && is_archive(&name);
        let me = self.clone();
        self.rt.spawn_blocking(move || {
            if scan && defender_flags(&path) {
                let text = "Windows Security found a threat in this file and dealt with it.";
                let mut st = me.lock();
                if let Some(item) = st.items.get_mut(&id) {
                    item.error = Some(text.into());
                    let item = item.clone();
                    me.persist(&item);
                }
                drop(st);
                me.emit(ManagerEvent::Changed(id));
                return me.notice(id, "threat", &name, &path, text);
            }
            if extract && path.is_file() {
                match unpack(&path) {
                    Ok(dest) => {
                        let gone = s.delete_archive_after_extract && recycle(&path);
                        let text = if gone { "Unpacked. The archive went to the Recycle Bin." } else { "Unpacked next to the archive." };
                        return me.notice(id, "unpacked", &name, &dest, text);
                    }
                    Err(e) => return me.notice(id, "done", &name, &path, format!("{e}.")),
                }
            }
            me.notice(id, "done", &name, &path, "");
        });
    }

    /// Stops a download whose drive would drop under the free-space floor.
    fn check_space_locked(&self, st: &mut State, id: i64) {
        let floor = st.settings.min_free_space_mb.saturating_mul(MB);
        if floor == 0 || st.no_space.contains_key(&id) {
            return;
        }
        let Some(item) = st.items.get(&id) else { return };
        let left = item.size.map(|s| s.saturating_sub(item.downloaded)).unwrap_or(0);
        let Some(free) = free_bytes(&item.folder) else { return };
        if free >= left.saturating_add(floor) {
            return;
        }
        let reason = format!(
            "Not enough free space. It needs {} and the drive has {}, with {} kept free.",
            human_size(left),
            human_size(free),
            human_size(floor)
        );
        if let Some(run) = st.running.get(&id) {
            let _ = run.cmd.send(RunCmd::Pause);
            st.no_space.insert(id, reason);
        }
    }

    fn schedule_tick(self: &Arc<Self>, now: NaiveDateTime) {
        let mut st = self.lock();
        let last = st.last_tick.replace(now).unwrap_or(now - chrono::Duration::minutes(1));
        let actions = schedule_actions(&st.queues, last, now);
        if actions.is_empty() {
            return;
        }
        for (qid, action) in actions {
            match action {
                ScheduleAction::Start => self.start_queue_locked(&mut st, qid),
                ScheduleAction::Stop => self.stop_queue_locked(&mut st, qid),
            }
        }
        self.pump(&mut st);
    }

    fn view(st: &State, item: &DownloadItem) -> DownloadView {
        let run = st.running.get(&item.id);
        let speed = run.map(|r| r.speed).unwrap_or(0);
        let eta_secs = match (item.size, speed) {
            (Some(size), s) if s > 0 && item.status.is_running() => Some(size.saturating_sub(item.downloaded) / s),
            _ => None,
        };
        DownloadView {
            id: item.id,
            name: item.name.clone(),
            folder: item.folder.clone(),
            url: item.url.clone(),
            size: item.size,
            downloaded: item.downloaded,
            status: item.status,
            error: item.error.clone(),
            speed,
            eta_secs,
            date_added: item.date_added,
            date_finished: item.date_finished,
            queue_id: item.queue_id,
            category_id: item.category_id,
            connections: run.map(|r| r.connections).unwrap_or(0),
            speed_limit: item.speed_limit,
        }
    }

    pub fn list(&self) -> Vec<DownloadView> {
        let st = self.lock();
        st.items.values().map(|item| Self::view(&st, item)).collect()
    }

    pub fn get(&self, id: i64) -> Option<DownloadView> {
        let st = self.lock();
        st.items.get(&id).map(|item| Self::view(&st, item))
    }

    /// The stored record, for the checksum dialog and tests.
    pub fn item(&self, id: i64) -> Option<DownloadItem> {
        self.lock().items.get(&id).cloned()
    }

    pub async fn probe(&self, url: &str, headers: Vec<(String, String)>) -> Result<ProbeInfo> {
        let (engine, folder) = {
            let mut st = self.lock();
            (self.engine(&mut st)?, st.settings.default_folder())
        };
        let mut spec = JobSpec::new(url, folder);
        spec.headers = headers;
        engine.probe(&spec).await.map_err(|e| anyhow!("{e}"))
    }

    fn engine(&self, st: &mut State) -> Result<Engine> {
        if let Some(engine) = &st.engine {
            return Ok(engine.clone());
        }
        let defaults = EngineConfig::default();
        let s = &st.settings;
        let cfg = EngineConfig {
            user_agent: if s.user_agent.trim().is_empty() { defaults.user_agent } else { s.user_agent.clone() },
            ignore_tls_errors: s.ignore_ssl_certificates,
            proxy: s.proxy.clone(),
            doh_url: s.doh_url(),
            global_limit: self.limiter.clone(),
            ..defaults
        };
        let engine = Engine::new(cfg).map_err(|e| anyhow!("{e}"))?;
        st.engine = Some(engine.clone());
        Ok(engine)
    }

    // ---- adding ----

    pub fn add(self: &Arc<Self>, req: AddRequest) -> Result<i64> {
        let url = req.url.trim().to_string();
        let parsed = url::Url::parse(&url).map_err(|_| anyhow!("that link is not valid"))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            bail!("only http and https links can be downloaded");
        }
        let referrer = req.referrer.clone().filter(|r| !r.trim().is_empty());
        let key = (url.clone(), referrer.clone().unwrap_or_default());

        let (id, queue_to_start) = {
            let mut st = self.lock();
            let now = Instant::now();
            st.recent.retain(|r| now.duration_since(r.at) < DUPLICATE_WINDOW);
            if let Some(found) = st.recent.iter().find(|r| r.key == key)
                && st.items.contains_key(&found.id)
            {
                return Ok(found.id);
            }
            if st.settings.skip_already_downloaded && !req.again {
                let done = st.items.values().rev().find(|i| {
                    i.status == Status::Completed && i.url == url && Path::new(&i.folder).join(&i.name).is_file()
                });
                if let Some(done) = done {
                    let path = Path::new(&done.folder).join(&done.name);
                    self.notice(done.id, "already", &done.name, &path, "You already downloaded this, and the file is still there.");
                    return Ok(done.id);
                }
            }

            let explicit_name = req.name.as_deref().or(req.suggested_name.as_deref()).map(str::trim).filter(|n| !n.is_empty());
            let name_locked = explicit_name.is_some();
            let name = explicit_name.map(sanitize_name).or_else(|| name_from_url(&url)).unwrap_or_else(|| "download".into());

            let by_id = req.category_id.and_then(|cid| st.categories.iter().find(|c| c.id == cid));
            let matched = by_id.or_else(|| {
                if st.settings.use_category_by_default { st.categories.iter().find(|c| c.matches(&name, &url)) } else { None }
            });
            let folder = req
                .folder
                .clone()
                .filter(|f| !f.trim().is_empty())
                .or_else(|| matched.filter(|c| c.use_folder && !c.folder.is_empty()).map(|c| c.folder.clone()))
                .unwrap_or_else(|| st.settings.default_download_folder.clone());

            let queue_id = if req.start_queue { Some(req.queue_id.unwrap_or(MAIN_QUEUE_ID)) } else { req.queue_id }
                .filter(|q| st.queues.iter().any(|queue| queue.id == *q));

            let mut item = DownloadItem::new(url, folder, name);
            item.kind = req.kind;
            item.headers = req.headers;
            item.referrer = referrer;
            item.name_locked = name_locked;
            item.category_id = matched.map(|c| c.id);
            item.queue_id = queue_id;
            let rule = st.settings.rule_for(&item.url).cloned().unwrap_or_default();
            let text = |s: &str| Some(s.trim().to_string()).filter(|s| !s.is_empty());
            item.basic_auth = req
                .basic_auth
                .clone()
                .filter(|(user, _)| !user.trim().is_empty())
                .or_else(|| text(&rule.username).map(|user| (user, rule.password.clone())));
            item.user_agent = req.user_agent.as_deref().and_then(text).or_else(|| text(&rule.user_agent));
            item.threads = req.threads.filter(|t| *t > 0).or((rule.threads > 0).then_some(rule.threads));
            item.speed_limit = if req.speed_limit > 0 { req.speed_limit } else { rule.speed_limit };
            let id = self.store.insert_download(&item)?;
            item.id = id;
            st.items.insert(id, item);

            if let Some(qid) = queue_id
                && let Some(queue) = st.queues.iter_mut().find(|q| q.id == qid)
            {
                queue.items.push(id);
                let queue = queue.clone();
                self.persist_queue(&queue);
            }
            st.recent.push(Recent { key, at: now, id });
            (id, if req.start_queue { queue_id } else { None })
        };
        self.emit(ManagerEvent::Added(id));
        if queue_to_start.is_some() {
            self.emit(ManagerEvent::QueuesChanged);
        }
        match queue_to_start {
            Some(qid) => self.start_queue(qid),
            None if req.start => self.start(id),
            None => {}
        }
        Ok(id)
    }

    /// Asks the window to show its add dialog for this link.
    pub fn request_show(&self) {
        self.emit(ManagerEvent::ShowRequested);
    }

    /// Asks the window to close the app.
    pub fn request_exit(&self) {
        self.emit(ManagerEvent::ExitRequested);
    }

    pub fn request_add(&self, req: AddRequest) {
        self.emit(ManagerEvent::AddRequested(Box::new(req)));
    }

    /// Records a file the browser already finished, without downloading it again.
    pub fn adopt_file(self: &Arc<Self>, path: &Path, url: Option<String>, referrer: Option<String>) -> Result<i64> {
        let meta = std::fs::metadata(path).map_err(|_| anyhow!("that file does not exist"))?;
        if !meta.is_file() {
            bail!("that path is not a file");
        }
        let folder = path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| anyhow!("that file has no name"))?;
        let id = {
            let mut st = self.lock();
            if let Some(existing) = st.items.values().find(|i| {
                i.status == Status::Completed && i.name.eq_ignore_ascii_case(&name) && i.folder.eq_ignore_ascii_case(&folder)
            }) {
                return Ok(existing.id);
            }
            let url = url
                .filter(|u| !u.trim().is_empty())
                .or_else(|| url::Url::from_file_path(path).ok().map(|u| u.to_string()))
                .unwrap_or_default();
            let mut item = DownloadItem::new(url, folder, name.clone());
            item.referrer = referrer.filter(|r| !r.trim().is_empty());
            item.name_locked = true;
            item.status = Status::Completed;
            item.size = Some(meta.len());
            item.downloaded = meta.len();
            item.date_finished = Some(now_ms());
            item.date_started = item.date_finished;
            item.category_id = st.categories.iter().find(|c| c.matches(&name, &item.url)).map(|c| c.id);
            let id = self.store.insert_download(&item)?;
            item.id = id;
            st.items.insert(id, item);
            id
        };
        self.emit(ManagerEvent::Added(id));
        Ok(id)
    }

    // ---- controlling ----

    pub fn start(self: &Arc<Self>, id: i64) {
        let mut st = self.lock();
        if self.queue_item_locked(&mut st, id) {
            st.manual.insert(id);
        }
        self.pump(&mut st);
    }

    pub fn resume(self: &Arc<Self>, id: i64) {
        self.start(id);
    }

    /// Marks a startable item as waiting for a slot. False when it cannot start.
    fn queue_item_locked(&self, st: &mut State, id: i64) -> bool {
        let Some(item) = st.items.get_mut(&id) else { return false };
        match item.status {
            Status::Added | Status::Paused | Status::Failed => {
                item.status = Status::Queued;
                item.error = None;
                let item = item.clone();
                self.persist(&item);
                self.emit(ManagerEvent::Changed(id));
                true
            }
            Status::Queued => true,
            _ => false,
        }
    }

    pub fn pause(self: &Arc<Self>, id: i64) {
        let mut st = self.lock();
        self.pause_locked(&mut st, id);
        self.pump(&mut st);
    }

    fn pause_locked(&self, st: &mut State, id: i64) {
        if let Some(run) = st.running.get(&id) {
            let _ = run.cmd.send(RunCmd::Pause);
            return;
        }
        st.manual.remove(&id);
        if let Some(item) = st.items.get_mut(&id)
            && item.status == Status::Queued
        {
            item.status = Status::Paused;
            let item = item.clone();
            self.persist(&item);
            self.emit(ManagerEvent::Changed(id));
        }
    }

    pub fn remove(self: &Arc<Self>, id: i64, delete_file: bool) -> Result<()> {
        let (item, join, delete_partial) = {
            let mut st = self.lock();
            let Some(item) = st.items.remove(&id) else { return Ok(()) };
            self.store.delete_download(id)?;
            st.manual.remove(&id);
            let mut changed = Vec::new();
            for queue in st.queues.iter_mut().filter(|q| q.items.contains(&id)) {
                queue.items.retain(|i| *i != id);
                changed.push(queue.clone());
            }
            for queue in &changed {
                self.persist_queue(queue);
            }
            let join = st.running.remove(&id).and_then(|run| {
                let _ = run.cmd.send(RunCmd::Pause);
                run.join
            });
            let delete_partial = st.settings.delete_partial_file_on_cancellation;
            self.pump(&mut st);
            if !changed.is_empty() {
                self.emit(ManagerEvent::QueuesChanged);
            }
            (item, join, delete_partial)
        };
        self.emit(ManagerEvent::Removed(id));

        let suffix = self.lock().settings.temp_suffix.clone();
        self.rt.spawn(async move {
            let mut doomed: Vec<PathBuf> = Vec::new();
            if item.status == Status::Completed {
                if delete_file {
                    doomed.push(Path::new(&item.folder).join(&item.name));
                }
            } else if delete_file || delete_partial {
                let live_state = match join {
                    Some(join) => join.await.ok().flatten(),
                    None => None,
                };
                if let Some(state) = live_state.or(item.resume.clone()) {
                    doomed.push(state.working_path);
                }
                doomed.push(Path::new(&item.folder).join(format!("{}{}", item.name, suffix)));
                if delete_file {
                    doomed.push(Path::new(&item.folder).join(&item.name));
                }
            }
            for path in doomed {
                let _ = tokio::fs::remove_file(path).await;
            }
        });
        Ok(())
    }

    pub fn start_queue(self: &Arc<Self>, qid: i64) {
        let mut st = self.lock();
        self.start_queue_locked(&mut st, qid);
        self.pump(&mut st);
    }

    fn start_queue_locked(&self, st: &mut State, qid: i64) {
        let Some(queue) = st.queues.iter().find(|q| q.id == qid) else { return };
        let ids = queue.items.clone();
        st.active_queues.insert(qid);
        for id in &ids {
            self.queue_item_locked(st, *id);
        }
        // Starting a queue with nothing left to do must not fire its power action.
        let has_work =
            ids.iter().any(|id| st.items.get(id).is_some_and(|i| matches!(i.status, Status::Queued | Status::Downloading | Status::Retrying)));
        if !has_work {
            st.active_queues.remove(&qid);
        }
    }

    pub fn stop_queue(self: &Arc<Self>, qid: i64) {
        let mut st = self.lock();
        self.stop_queue_locked(&mut st, qid);
        self.pump(&mut st);
    }

    fn stop_queue_locked(&self, st: &mut State, qid: i64) {
        st.active_queues.remove(&qid);
        let ids = st.queues.iter().find(|q| q.id == qid).map(|q| q.items.clone()).unwrap_or_default();
        for id in ids {
            if !st.manual.contains(&id) || st.items.get(&id).is_some_and(|i| i.status == Status::Queued) {
                self.pause_locked(st, id);
            }
        }
    }

    pub fn stop_all(self: &Arc<Self>) {
        self.pause_all();
    }

    pub fn pause_all(self: &Arc<Self>) {
        let mut st = self.lock();
        st.active_queues.clear();
        st.manual.clear();
        let ids: Vec<i64> = st.items.values().filter(|i| matches!(i.status, Status::Queued | Status::Downloading | Status::Retrying)).map(|i| i.id).collect();
        for id in ids {
            self.pause_locked(&mut st, id);
        }
        self.pump(&mut st);
    }

    pub fn resume_all(self: &Arc<Self>) {
        let mut st = self.lock();
        let ids: Vec<i64> = st.items.values().filter(|i| i.status == Status::Paused).map(|i| i.id).collect();
        for id in ids {
            if self.queue_item_locked(&mut st, id) {
                st.manual.insert(id);
            }
        }
        self.pump(&mut st);
    }

    // ---- scheduling ----

    fn next_startable(st: &State) -> Option<i64> {
        let queued = |id: &i64| st.items.get(id).is_some_and(|i| i.status == Status::Queued);
        let mut manual: Vec<i64> = st.manual.iter().copied().filter(queued).collect();
        manual.sort_unstable();
        if let Some(id) = manual.first() {
            return Some(*id);
        }
        for queue in st.queues.iter().filter(|q| st.active_queues.contains(&q.id)) {
            let running = st.running.keys().filter(|id| st.items.get(id).and_then(|i| i.queue_id) == Some(queue.id)).count();
            if queue.max_concurrent != 0 && running >= queue.max_concurrent as usize {
                continue;
            }
            if let Some(id) = queue.items.iter().copied().find(queued) {
                return Some(id);
            }
        }
        None
    }

    fn pump(self: &Arc<Self>, st: &mut State) {
        loop {
            let max = st.settings.max_concurrent_downloads as usize;
            if max != 0 && st.running.len() >= max {
                break;
            }
            let Some(id) = Self::next_startable(st) else { break };
            self.launch(st, id);
        }
        let busy: Vec<i64> = st
            .active_queues
            .iter()
            .copied()
            .filter(|qid| {
                st.queues.iter().find(|q| q.id == *qid).is_some_and(|q| {
                    q.items.iter().any(|id| st.items.get(id).is_some_and(|i| matches!(i.status, Status::Queued | Status::Downloading | Status::Retrying)))
                })
            })
            .collect();
        let emptied: Vec<i64> = st.active_queues.iter().copied().filter(|qid| !busy.contains(qid)).collect();
        st.active_queues.retain(|qid| busy.contains(qid));
        for qid in emptied {
            let Some(queue) = st.queues.iter().find(|q| q.id == qid) else { continue };
            let failed = queue.items.iter().any(|id| st.items.get(id).is_some_and(|i| i.status == Status::Failed));
            if queue.power_action != PowerAction::None && !failed {
                self.emit(ManagerEvent::QueueFinished { id: qid, name: queue.name.clone(), action: queue.power_action });
            }
        }
        self.awake.set(!st.running.is_empty());
    }

    fn fail_locked(&self, st: &mut State, id: i64, message: String) {
        st.manual.remove(&id);
        if let Some(item) = st.items.get_mut(&id) {
            item.status = Status::Failed;
            item.error = Some(message);
            let item = item.clone();
            self.persist(&item);
            self.emit(ManagerEvent::Changed(id));
        }
    }

    fn launch(self: &Arc<Self>, st: &mut State, id: i64) {
        st.manual.remove(&id);
        let Some(item) = st.items.get(&id).cloned() else { return };
        let engine = match self.engine(st) {
            Ok(engine) => engine,
            Err(e) => return self.fail_locked(st, id, e.to_string()),
        };
        if let Err(e) = std::fs::create_dir_all(&item.folder) {
            return self.fail_locked(st, id, format!("could not create the folder: {e}"));
        }
        let spec = job_spec(&item, &st.settings);
        let _guard = self.rt.enter();
        let handle = engine.start(spec, item.resume.clone());
        let (cmd, cmd_rx) = mpsc::unbounded_channel();
        let join = self.rt.spawn(run_job(self.clone(), id, handle, cmd_rx));
        st.running.insert(id, Running { cmd, speed: 0, connections: 0, join: Some(join) });
        if let Some(item) = st.items.get_mut(&id) {
            item.status = Status::Downloading;
            item.error = None;
            item.date_started = Some(now_ms());
            let item = item.clone();
            self.persist(&item);
        }
        self.emit(ManagerEvent::Changed(id));
    }

    fn on_event(self: &Arc<Self>, id: i64, event: Event, last_save: &mut Instant) -> Option<Option<ResumeState>> {
        let mut st = self.lock();
        let st = &mut *st;
        if !st.items.contains_key(&id) {
            return match event {
                Event::Paused(state) | Event::Failed { state, .. } => Some(Some(state)),
                Event::Completed(_) => Some(None),
                _ => None,
            };
        }
        match event {
            Event::Probed(info) => {
                let suffix = st.settings.append_extension_to_incomplete_downloads.then(|| st.settings.temp_suffix.clone());
                let item = st.items.get_mut(&id)?;
                if !item.name_locked {
                    item.name = info.file_name.clone();
                }
                if info.total.is_some() {
                    item.size = info.total;
                }
                let working = Path::new(&item.folder).join(format!("{}{}", info.file_name, suffix.unwrap_or_default()));
                let resume = item.resume.get_or_insert_with(|| ResumeState { working_path: working, ..Default::default() });
                resume.file_name = info.file_name;
                resume.total = info.total;
                resume.resumable = info.resumable;
                resume.validator = info.validator;
                let item = item.clone();
                self.persist(&item);
                self.emit(ManagerEvent::Changed(id));
                self.check_space_locked(st, id);
                None
            }
            Event::Progress(p) => {
                let item = st.items.get_mut(&id)?;
                let was_retrying = item.status == Status::Retrying;
                item.downloaded = p.downloaded;
                if p.total.is_some() {
                    item.size = p.total;
                }
                if was_retrying {
                    item.status = Status::Downloading;
                    item.error = None;
                }
                if let Some(resume) = item.resume.as_mut() {
                    resume.parts = p.parts;
                }
                let (downloaded, size) = (item.downloaded, item.size);
                if last_save.elapsed() >= SAVE_EVERY {
                    *last_save = Instant::now();
                    let item = item.clone();
                    self.persist(&item);
                }
                if let Some(run) = st.running.get_mut(&id) {
                    run.speed = p.speed;
                    run.connections = p.active_connections;
                }
                if was_retrying {
                    self.emit(ManagerEvent::Changed(id));
                }
                self.emit(ManagerEvent::Progress { id, downloaded, total: size, speed: p.speed });
                None
            }
            Event::Retrying { error, .. } => {
                let item = st.items.get_mut(&id)?;
                item.status = Status::Retrying;
                item.error = Some(error.to_string());
                let item = item.clone();
                self.persist(&item);
                self.emit(ManagerEvent::Changed(id));
                None
            }
            Event::Paused(state) => {
                if let Some(reason) = st.no_space.remove(&id) {
                    st.manual.remove(&id);
                    let item = st.items.get(&id)?;
                    let (name, folder) = (item.name.clone(), item.folder.clone());
                    self.emit(ManagerEvent::Done { id, ok: false });
                    self.notice(id, "nospace", &name, Path::new(&folder), reason.clone());
                    self.finish_locked(st, id, |item| {
                        item.status = Status::Failed;
                        item.error = Some(reason);
                        apply_state(item, state);
                    });
                    return Some(None);
                }
                self.finish_locked(st, id, |item| {
                    item.status = Status::Paused;
                    apply_state(item, state);
                });
                Some(None)
            }
            Event::Failed { error, state } => {
                self.emit(ManagerEvent::Done { id, ok: false });
                st.no_space.remove(&id);
                if let Some(item) = st.items.get(&id) {
                    self.notice(id, "failed", &item.name, Path::new(&item.folder), error.to_string());
                }
                self.finish_locked(st, id, |item| {
                    item.status = Status::Failed;
                    item.error = Some(error.to_string());
                    apply_state(item, state);
                });
                Some(None)
            }
            Event::Completed(done) => {
                let mut queues_changed = Vec::new();
                for queue in st.queues.iter_mut().filter(|q| q.items.contains(&id)) {
                    queue.items.retain(|i| *i != id);
                    queues_changed.push(queue.clone());
                }
                for queue in &queues_changed {
                    self.persist_queue(queue);
                }
                self.finish_locked(st, id, |item| {
                    item.status = Status::Completed;
                    item.error = None;
                    item.resume = None;
                    item.size = Some(done.size);
                    item.downloaded = done.size;
                    item.name = done.file_name.clone();
                    if let Some(parent) = done.path.parent() {
                        item.folder = parent.to_string_lossy().into_owned();
                    }
                    item.date_finished = Some(now_ms());
                });
                self.emit(ManagerEvent::Done { id, ok: true });
                self.after_finish(id, done.path.clone());
                if !queues_changed.is_empty() {
                    self.emit(ManagerEvent::QueuesChanged);
                }
                Some(None)
            }
        }
    }

    fn finish_locked(self: &Arc<Self>, st: &mut State, id: i64, change: impl FnOnce(&mut DownloadItem)) {
        st.running.remove(&id);
        if let Some(item) = st.items.get_mut(&id) {
            change(item);
            let item = item.clone();
            self.persist(&item);
            self.emit(ManagerEvent::Changed(id));
        }
        self.pump(st);
    }

    // ---- settings, queues, categories ----

    pub fn settings(&self) -> Settings {
        self.lock().settings.clone()
    }

    pub fn update_settings(self: &Arc<Self>, new: Settings) -> Result<()> {
        if new.thread_count == 0 {
            bail!("the number of connections must be at least 1");
        }
        if new.api_key.trim().is_empty() {
            bail!("the api key cannot be empty");
        }
        new.save(&self.data_dir.join("settings.json"))?;
        {
            let mut st = self.lock();
            let old = std::mem::replace(&mut st.settings, new.clone());
            if old.proxy != new.proxy
                || old.user_agent != new.user_agent
                || old.ignore_ssl_certificates != new.ignore_ssl_certificates
                || old.dns_over_https != new.dns_over_https
            {
                st.engine = None;
            }
            self.limiter.set_rate(new.effective_speed_limit());
            self.pump(&mut st);
        }
        self.emit(ManagerEvent::SettingsChanged);
        Ok(())
    }

    pub fn queues(&self) -> Vec<Queue> {
        self.lock().queues.clone()
    }

    /// Queues that are started and still have work.
    pub fn active_queue_ids(&self) -> Vec<i64> {
        self.lock().active_queues.iter().copied().collect()
    }

    /// A negative id creates a new queue. Returns the saved id.
    pub fn save_queue(self: &Arc<Self>, mut queue: Queue) -> Result<i64> {
        let mut st = self.lock();
        if queue.id < 0 {
            queue.id = self.store.next_queue_id()?;
        }
        if queue.name.trim().is_empty() {
            bail!("a queue needs a name");
        }
        let old_items = st.queues.iter().find(|q| q.id == queue.id).map(|q| q.items.clone()).unwrap_or_default();
        let mut touched = Vec::new();
        for id in old_items.iter().filter(|i| !queue.items.contains(i)) {
            if let Some(item) = st.items.get_mut(id).filter(|i| i.queue_id == Some(queue.id)) {
                item.queue_id = None;
                touched.push(item.clone());
            }
        }
        for id in &queue.items {
            if let Some(item) = st.items.get_mut(id).filter(|i| i.queue_id != Some(queue.id)) {
                item.queue_id = Some(queue.id);
                touched.push(item.clone());
            }
        }
        self.store.save_queue(&queue)?;
        let qid = queue.id;
        match st.queues.iter_mut().find(|q| q.id == qid) {
            Some(slot) => *slot = queue,
            None => st.queues.push(queue),
        }
        for item in &touched {
            self.persist(item);
            self.emit(ManagerEvent::Changed(item.id));
        }
        self.pump(&mut st);
        drop(st);
        self.emit(ManagerEvent::QueuesChanged);
        Ok(qid)
    }

    pub fn delete_queue(self: &Arc<Self>, id: i64) -> Result<()> {
        if id == MAIN_QUEUE_ID {
            bail!("the Main queue cannot be deleted");
        }
        let mut st = self.lock();
        self.store.delete_queue(id)?;
        st.queues.retain(|q| q.id != id);
        st.active_queues.remove(&id);
        let mut touched = Vec::new();
        for item in st.items.values_mut().filter(|i| i.queue_id == Some(id)) {
            item.queue_id = None;
            touched.push(item.clone());
        }
        for item in &touched {
            self.persist(item);
            self.emit(ManagerEvent::Changed(item.id));
        }
        drop(st);
        self.emit(ManagerEvent::QueuesChanged);
        Ok(())
    }

    /// Sets one download's own speed limit (bytes per second, 0 = none). A running download
    /// picks it up at once.
    pub fn set_speed_limit(self: &Arc<Self>, id: i64, rate: u64) {
        let mut st = self.lock();
        let Some(item) = st.items.get_mut(&id) else { return };
        item.speed_limit = rate;
        let item = item.clone();
        self.persist(&item);
        if let Some(run) = st.running.get(&id) {
            let _ = run.cmd.send(RunCmd::SetSpeedLimit(rate));
        }
        drop(st);
        self.emit(ManagerEvent::Changed(id));
    }

    /// Moves downloads to the end of a queue. Finished downloads are left alone.
    pub fn move_to_queue(self: &Arc<Self>, ids: &[i64], qid: i64) -> Result<()> {
        let mut st = self.lock();
        if !st.queues.iter().any(|q| q.id == qid) {
            bail!("that queue does not exist");
        }
        let mut touched_queues: HashSet<i64> = HashSet::new();
        for id in ids {
            let Some(item) = st.items.get(id).filter(|i| i.status != Status::Completed) else { continue };
            let previous = item.queue_id;
            if let Some(prev) = previous
                && let Some(queue) = st.queues.iter_mut().find(|q| q.id == prev)
            {
                queue.items.retain(|i| i != id);
                touched_queues.insert(prev);
            }
            if let Some(queue) = st.queues.iter_mut().find(|q| q.id == qid) {
                queue.items.retain(|i| i != id);
                queue.items.push(*id);
                touched_queues.insert(qid);
            }
            if let Some(item) = st.items.get_mut(id) {
                item.queue_id = Some(qid);
                let item = item.clone();
                self.persist(&item);
                self.emit(ManagerEvent::Changed(*id));
            }
        }
        for queue in st.queues.iter().filter(|q| touched_queues.contains(&q.id)) {
            self.persist_queue(queue);
        }
        self.pump(&mut st);
        drop(st);
        self.emit(ManagerEvent::QueuesChanged);
        Ok(())
    }

    pub fn categories(&self) -> Vec<Category> {
        self.lock().categories.clone()
    }

    /// A negative id creates a new category. Returns the saved id.
    pub fn save_category(&self, mut category: Category) -> Result<i64> {
        if category.id < 0 {
            category.id = self.store.next_category_id()?;
        }
        if category.name.trim().is_empty() {
            bail!("a category needs a name");
        }
        self.store.save_category(&category)?;
        let id = category.id;
        let mut st = self.lock();
        match st.categories.iter_mut().find(|c| c.id == id) {
            Some(slot) => *slot = category,
            None => st.categories.push(category),
        }
        st.categories.sort_by_key(|c| c.id);
        Ok(id)
    }

    pub fn delete_category(&self, id: i64) -> Result<()> {
        self.store.delete_category(id)?;
        let mut st = self.lock();
        st.categories.retain(|c| c.id != id);
        let mut touched = Vec::new();
        for item in st.items.values_mut().filter(|i| i.category_id == Some(id)) {
            item.category_id = None;
            touched.push(item.clone());
        }
        for item in &touched {
            self.persist(item);
            self.emit(ManagerEvent::Changed(item.id));
        }
        Ok(())
    }

}

fn apply_state(item: &mut DownloadItem, state: ResumeState) {
    item.downloaded = state.downloaded();
    if state.total.is_some() {
        item.size = state.total;
    }
    item.resume = Some(state);
}

fn job_spec(item: &DownloadItem, s: &Settings) -> JobSpec {
    let mut spec = JobSpec::new(&item.url, &item.folder);
    spec.kind = item.kind;
    spec.headers = item.headers.clone();
    spec.referrer = item.referrer.clone();
    spec.user_agent = item.user_agent.clone();
    spec.basic_auth = item.basic_auth.clone();
    spec.file_name = match &item.resume {
        Some(resume) if !resume.file_name.is_empty() => Some(resume.file_name.clone()),
        _ if item.name_locked => Some(item.name.clone()),
        _ => None,
    };
    spec.threads = item.threads.unwrap_or(s.thread_count).max(1);
    spec.min_part_size = s.min_part_size;
    spec.dynamic_parts = s.dynamic_part_creation;
    spec.max_retries = s.max_retry_count;
    spec.speed_limit = item.speed_limit;
    spec.sparse = s.use_sparse_file_allocation;
    spec.temp_suffix = s.append_extension_to_incomplete_downloads.then(|| s.temp_suffix.clone());
    spec.apply_server_mtime = s.use_server_last_modified_time;
    spec.on_exists = if s.replace_same_name { OnExists::Overwrite } else { OnExists::Number };
    spec
}

async fn run_job(
    manager: Arc<Manager>,
    id: i64,
    mut handle: JobHandle,
    mut commands: mpsc::UnboundedReceiver<RunCmd>,
) -> Option<ResumeState> {
    let mut last_save = Instant::now();
    let mut commands_open = true;
    loop {
        tokio::select! {
            event = handle.events.recv() => {
                match event {
                    Some(event) => {
                        if let Some(state) = manager.on_event(id, event, &mut last_save) {
                            return state_of(&manager, id, state);
                        }
                    }
                    None => {
                        let mut st = manager.lock();
                        if st.running.contains_key(&id) {
                            manager.finish_locked(&mut st, id, |item| {
                                item.status = Status::Failed;
                                item.error = Some("the download stopped unexpectedly".into());
                            });
                        }
                        return None;
                    }
                }
            }
            command = commands.recv(), if commands_open => {
                match command {
                    Some(RunCmd::Pause) => handle.pause(),
                    Some(RunCmd::SetSpeedLimit(rate)) => handle.set_speed_limit(rate),
                    None => commands_open = false,
                }
            }
        }
    }
}

fn state_of(_manager: &Arc<Manager>, _id: i64, state: Option<ResumeState>) -> Option<ResumeState> {
    state
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.awake.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 23).unwrap().and_hms_opt(h, m, 0).unwrap()
    }

    async fn open() -> (tempfile::TempDir, Arc<Manager>) {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::open(dir.path().to_path_buf()).unwrap();
        (dir, manager)
    }

    fn req(url: &str) -> AddRequest {
        AddRequest { url: url.into(), ..Default::default() }
    }

    #[tokio::test]
    async fn open_creates_defaults() {
        let (_dir, m) = open().await;
        assert_eq!(m.categories().len(), 6);
        assert_eq!(m.queues().len(), 1);
        assert_eq!(m.queues()[0].name, "Main");
        assert_eq!(m.queues()[0].max_concurrent, 2);
        assert_eq!(m.settings().api_key.len(), 32);
    }

    #[tokio::test]
    async fn add_uses_category_folder_and_name() {
        let (_dir, m) = open().await;
        let mut rx = m.subscribe();
        let id = m.add(req("https://example.com/files/Movie%20One.mkv?token=1")).unwrap();
        let view = m.get(id).unwrap();
        assert_eq!(view.name, "Movie One.mkv");
        assert!(view.folder.ends_with("Videos"), "{}", view.folder);
        assert_eq!(view.category_id, Some(2));
        assert_eq!(view.status, Status::Added);
        assert!(matches!(rx.try_recv(), Ok(ManagerEvent::Added(i)) if i == id));
    }

    #[tokio::test]
    async fn explicit_folder_and_name_win() {
        let (_dir, m) = open().await;
        let id = m
            .add(AddRequest { url: "https://example.com/a.zip".into(), folder: Some("D:\\Elsewhere".into()), name: Some("x:y.zip".into()), ..Default::default() })
            .unwrap();
        let view = m.get(id).unwrap();
        assert_eq!(view.folder, "D:\\Elsewhere");
        assert_eq!(view.name, "x_y.zip");
    }

    #[tokio::test]
    async fn duplicate_guard_returns_earlier_id() {
        let (_dir, m) = open().await;
        let a = m.add(req("https://example.com/a.zip")).unwrap();
        let b = m.add(req("https://example.com/a.zip")).unwrap();
        assert_eq!(a, b);
        let other_referrer = m.add(AddRequest { referrer: Some("https://example.com/page".into()), ..req("https://example.com/a.zip") }).unwrap();
        assert_ne!(a, other_referrer);
        assert_eq!(m.list().len(), 2);
    }

    #[tokio::test]
    async fn rejects_bad_links() {
        let (_dir, m) = open().await;
        assert!(m.add(req("not a link")).is_err());
        assert!(m.add(req("ftp://example.com/a")).is_err());
    }

    #[tokio::test]
    async fn queue_assignment_and_moves() {
        let (_dir, m) = open().await;
        let a = m.add(AddRequest { queue_id: Some(0), ..req("https://example.com/a.zip") }).unwrap();
        let b = m.add(req("https://example.com/b.zip")).unwrap();
        let qid = m.save_queue(Queue { id: -1, name: "Night".into(), ..Queue::main() }).unwrap();
        assert!(qid > 10);
        m.move_to_queue(&[a, b], qid).unwrap();
        assert_eq!(m.get(a).unwrap().queue_id, Some(qid));
        assert!(m.queues().iter().find(|q| q.id == 0).unwrap().items.is_empty());
        assert_eq!(m.queues().iter().find(|q| q.id == qid).unwrap().items, vec![a, b]);
        m.delete_queue(qid).unwrap();
        assert_eq!(m.get(b).unwrap().queue_id, None);
        assert!(m.delete_queue(0).is_err());
    }

    #[tokio::test]
    async fn downloading_items_come_back_paused() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = Store::open(&dir.path().join("sdm.db")).unwrap();
            let mut item = DownloadItem::new("https://example.com/a.zip", "D:\\Dl", "a.zip");
            item.status = Status::Downloading;
            store.insert_download(&item).unwrap();
        }
        let m = Manager::open(dir.path().to_path_buf()).unwrap();
        assert_eq!(m.list()[0].status, Status::Paused);
    }

    #[tokio::test]
    async fn uncategorised_items_get_a_category_on_open() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = Store::open(&dir.path().join("sdm.db")).unwrap();
            for name in ["setup.exe", "notes.xyz"] {
                let item = DownloadItem::new(format!("https://example.com/{name}"), "D:\\Dl", name);
                store.insert_download(&item).unwrap();
            }
        }
        let m = Manager::open(dir.path().to_path_buf()).unwrap();
        let by_name = |n: &str| m.list().into_iter().find(|v| v.name == n).unwrap().category_id;
        assert_eq!(by_name("setup.exe"), Some(1));
        assert_eq!(by_name("notes.xyz"), None);
        drop(m);
        let saved = Store::open(&dir.path().join("sdm.db")).unwrap().load_downloads().unwrap();
        assert!(saved.iter().any(|i| i.name == "setup.exe" && i.category_id == Some(1)));
    }

    #[tokio::test]
    async fn adopt_file_is_idempotent_and_remove_deletes() {
        let (dir, m) = open().await;
        let file = dir.path().join("report.pdf");
        std::fs::write(&file, b"12345").unwrap();
        let id = m.adopt_file(&file, Some("https://example.com/report.pdf".into()), None).unwrap();
        assert_eq!(m.adopt_file(&file, None, None).unwrap(), id);
        let view = m.get(id).unwrap();
        assert_eq!((view.status, view.size, view.downloaded), (Status::Completed, Some(5), 5));
        assert_eq!(view.category_id, Some(5));
        assert!(m.adopt_file(&dir.path().join("missing.bin"), None, None).is_err());
        m.remove(id, true).unwrap();
        assert!(m.get(id).is_none());
        for _ in 0..50 {
            if !file.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(!file.exists());
    }

    #[tokio::test]
    async fn settings_update_persists_and_notifies() {
        let (dir, m) = open().await;
        let mut rx = m.subscribe();
        let mut s = m.settings();
        s.thread_count = 3;
        m.update_settings(s.clone()).unwrap();
        assert!(matches!(rx.try_recv(), Ok(ManagerEvent::SettingsChanged)));
        assert_eq!(Settings::load(&dir.path().join("settings.json")).unwrap().thread_count, 3);
        s.thread_count = 0;
        assert!(m.update_settings(s).is_err());
    }

    #[test]
    fn schedule_window() {
        let mut queue = Queue::main();
        queue.schedule.enabled_start = true;
        queue.schedule.enabled_end = true;
        queue.schedule.start = Some("02:30".into());
        queue.schedule.end = Some("07:30".into());
        let queues = [queue.clone()];
        assert_eq!(schedule_actions(&queues, at(2, 29), at(2, 30)), vec![(0, ScheduleAction::Start)]);
        assert!(schedule_actions(&queues, at(2, 30), at(2, 31)).is_empty());
        assert_eq!(schedule_actions(&queues, at(7, 0), at(8, 0)), vec![(0, ScheduleAction::Stop)]);
        queue.schedule.days = vec![Weekday::Friday];
        assert!(schedule_actions(&[queue], at(2, 29), at(2, 30)).is_empty());
    }

    #[test]
    fn archive_and_scan_types() {
        assert!(is_archive("a.zip") && is_archive("B.RAR") && is_archive("c.7z") && is_archive("d.tar.gz"));
        assert!(!is_archive("log.gz") && !is_archive("movie.mkv"));
        assert!(wants_scan("setup.exe") && wants_scan("x.zip") && !wants_scan("song.flac"));
    }

    #[test]
    fn unpack_folder_is_unique() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unpack_folder(&dir.path().join("pack.tar.gz")), dir.path().join("pack"));
        std::fs::create_dir(dir.path().join("pack")).unwrap();
        assert_eq!(unpack_folder(&dir.path().join("pack.zip")), dir.path().join("pack (2)"));
        assert_eq!(unpack_folder(&dir.path().join(".zip")), dir.path().join(".zip"));
    }

    #[test]
    fn unpacks_with_windows_tar() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("inside.txt"), b"hi").unwrap();
        let archive = dir.path().join("box.zip");
        let made = hidden(r"C:\Windows\System32\tar.exe")
            .args(["-a", "-cf"])
            .arg(&archive)
            .arg("-C")
            .arg(dir.path())
            .arg("inside.txt")
            .status()
            .unwrap();
        assert!(made.success());
        let dest = unpack(&archive).unwrap();
        assert_eq!(dest, dir.path().join("box"));
        assert_eq!(std::fs::read(dest.join("inside.txt")).unwrap(), b"hi");
        std::fs::write(dir.path().join("broken.zip"), b"not a zip").unwrap();
        assert!(unpack(&dir.path().join("broken.zip")).is_err());
        assert!(!dir.path().join("broken").exists());
    }

    #[tokio::test]
    async fn already_downloaded_link_is_skipped() {
        let (dir, m) = open().await;
        let file = dir.path().join("tool.zip");
        std::fs::write(&file, b"1").unwrap();
        let first = m.adopt_file(&file, Some("https://example.com/tool.zip".into()), None).unwrap();
        let mut rx = m.subscribe();
        assert_eq!(m.add(req("https://example.com/tool.zip")).unwrap(), first);
        assert!(matches!(rx.try_recv(), Ok(ManagerEvent::Notice(n)) if n.kind == "already" && n.id == first));
        let again = m.add(AddRequest { again: true, ..req("https://example.com/tool.zip") }).unwrap();
        assert_ne!(again, first);
        std::fs::remove_file(&file).unwrap();
        assert_ne!(m.add(AddRequest { referrer: Some("https://example.com/".into()), ..req("https://example.com/tool.zip") }).unwrap(), first);
    }

    #[tokio::test]
    async fn old_finished_entries_leave_the_list_but_files_stay() {
        let (dir, m) = open().await;
        let file = dir.path().join("old.pdf");
        std::fs::write(&file, b"1").unwrap();
        let id = m.adopt_file(&file, None, None).unwrap();
        assert_eq!(m.clear_old_finished(now_ms(), true), 0);
        assert_eq!(m.clear_old_finished(now_ms() + 31 * DAY_MS, false), 0, "runs at most once an hour");
        assert_eq!(m.clear_old_finished(now_ms() + 31 * DAY_MS, true), 1);
        assert!(m.get(id).is_none());
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(file.exists());
    }

    #[test]
    fn replace_setting_picks_overwrite() {
        let item = DownloadItem::new("https://e.com/a.zip", "D:\\Dl", "a.zip");
        assert_eq!(job_spec(&item, &Settings::default()).on_exists, OnExists::Number);
        let s = Settings { replace_same_name: true, ..Settings::default() };
        assert_eq!(job_spec(&item, &s).on_exists, OnExists::Overwrite);
    }

    #[test]
    fn free_space_reads_a_missing_folder() {
        assert!(free_bytes(r"C:\definitely\not\here\sdm").is_some_and(|f| f > 0));
        assert_eq!(human_size(3 * 1024 * MB / 2), "1.5 GB");
        assert_eq!(human_size(5 * MB), "5 MB");
    }

    #[test]
    fn debug_hides_headers() {
        let r = AddRequest { headers: vec![("Cookie".into(), "secret-cookie".into())], ..req("https://e.com/a") };
        assert!(!format!("{r:?}").contains("secret-cookie"));
    }

    /// Needs the real engine bodies from sdm-engine.
    #[tokio::test]
    #[ignore = "needs a working sdm-engine"]
    async fn downloads_from_a_local_server() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let body = vec![7u8; 10_000];
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(&body);
            }
        });
        let (dir, m) = open().await;
        let id = m
            .add(AddRequest {
                url: format!("http://127.0.0.1:{port}/data.bin"),
                folder: Some(dir.path().join("out").to_string_lossy().into_owned()),
                start: true,
                ..Default::default()
            })
            .unwrap();
        for _ in 0..100 {
            if m.get(id).unwrap().status == Status::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(m.get(id).unwrap().status, Status::Completed);
    }
}
