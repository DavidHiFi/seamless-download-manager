//! Download engine for Seamless Download Manager.
//!
//! The public types in this file are the contract the rest of the app codes against.
//! Keep their names and fields; add fields only with a `Default`-friendly value.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

mod doh;
mod files;
mod hls;
mod job;
mod limiter;
mod meter;
mod names;
mod net;
pub use limiter::SpeedLimiter;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProxyConfig {
    /// Windows system proxy (reqwest picks it from the environment / registry).
    System,
    Direct,
    /// `http://host:port`, `socks5://host:port`, optional `user:pass@`.
    Manual { url: String },
}

#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub user_agent: String,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub ignore_tls_errors: bool,
    pub proxy: ProxyConfig,
    /// An https address that answers DNS questions (RFC 8484). None: Windows looks names up.
    pub doh_url: Option<String>,
    /// Shared across every job; rate 0 means unlimited.
    pub global_limit: Arc<SpeedLimiter>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36".into(),
            connect_timeout: Duration::from_secs(15),
            read_timeout: Duration::from_secs(30),
            ignore_tls_errors: false,
            proxy: ProxyConfig::System,
            doh_url: None,
            global_limit: Arc::new(SpeedLimiter::new(0)),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SourceKind {
    #[default]
    Http,
    Hls,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OnExists {
    /// Save as `name_1.ext`, `name_2.ext` ... (never prompt).
    #[default]
    Number,
    Overwrite,
    Fail,
}

#[derive(Clone, Debug)]
pub struct JobSpec {
    pub kind: SourceKind,
    pub url: String,
    /// Request headers from the browser (Cookie, Referer, ...). Host, Range, If-* and
    /// Accept-Encoding are dropped by the engine.
    pub headers: Vec<(String, String)>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub basic_auth: Option<(String, String)>,
    pub dest_dir: PathBuf,
    /// Final file name. `None`: from Content-Disposition, else the URL.
    pub file_name: Option<String>,
    pub threads: u32,
    pub min_part_size: u64,
    pub dynamic_parts: bool,
    pub max_retries: u32,
    /// Bytes per second for this job, 0 = none.
    pub speed_limit: u64,
    pub sparse: bool,
    /// Download into `<name><suffix>` and rename when done (e.g. ".sdm-part").
    pub temp_suffix: Option<String>,
    pub apply_server_mtime: bool,
    pub on_exists: OnExists,
}

impl JobSpec {
    pub fn new(url: impl Into<String>, dest_dir: impl Into<PathBuf>) -> Self {
        Self {
            kind: SourceKind::Http,
            url: url.into(),
            headers: Vec::new(),
            referrer: None,
            user_agent: None,
            basic_auth: None,
            dest_dir: dest_dir.into(),
            file_name: None,
            threads: 8,
            min_part_size: 512 * 1024,
            dynamic_parts: true,
            max_retries: 5,
            speed_limit: 0,
            sparse: true,
            temp_suffix: None,
            apply_server_mtime: false,
            on_exists: OnExists::Number,
        }
    }
}

/// One byte range. `to` is inclusive; `current` is the next byte to write.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Part {
    pub from: u64,
    pub to: u64,
    pub current: u64,
}

impl Part {
    pub fn done(&self) -> bool {
        self.current > self.to
    }
    pub fn remaining(&self) -> u64 {
        (self.to + 1).saturating_sub(self.current)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Validator {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Everything needed to continue a paused or interrupted job.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ResumeState {
    pub file_name: String,
    /// The path being written (temp name when `temp_suffix` is set).
    pub working_path: PathBuf,
    pub total: Option<u64>,
    pub resumable: bool,
    pub validator: Validator,
    pub parts: Vec<Part>,
    /// HLS: index of the next segment to fetch.
    pub hls_next_segment: Option<u64>,
}

impl ResumeState {
    pub fn downloaded(&self) -> u64 {
        self.parts.iter().map(|p| p.current - p.from).sum()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbeInfo {
    pub final_url: String,
    pub file_name: String,
    pub total: Option<u64>,
    pub resumable: bool,
    pub content_type: Option<String>,
    pub is_web_page: bool,
    pub needs_auth: bool,
    pub validator: Validator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
    /// Bytes per second, smoothed over about 3 s.
    pub speed: u64,
    pub active_connections: u32,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Completed {
    pub path: PathBuf,
    pub file_name: String,
    pub size: u64,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq)]
pub enum EngineError {
    #[error("the server said {status}")]
    Http { status: u16 },
    #[error("could not connect: {0}")]
    Network(String),
    #[error("could not write the file: {0}")]
    Disk(String),
    #[error("the file on the server changed, so the download restarts")]
    Changed,
    #[error("the file already exists")]
    Exists,
    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Probed(ProbeInfo),
    Progress(Progress),
    /// Sent once after `JobHandle::pause`; save it to resume later.
    Paused(ResumeState),
    /// Retrying after an error; `attempt` starts at 1.
    Retrying { attempt: u32, error: EngineError },
    Completed(Completed),
    /// Final failure. The state lets the user resume by hand.
    Failed { error: EngineError, state: ResumeState },
}

pub struct JobHandle {
    pub events: mpsc::UnboundedReceiver<Event>,
    control: mpsc::UnboundedSender<Control>,
}

pub(crate) enum Control {
    Pause,
    SetSpeedLimit(u64),
    SetThreads(u32),
}

impl JobHandle {
    pub fn pause(&self) {
        let _ = self.control.send(Control::Pause);
    }
    pub fn set_speed_limit(&self, bytes_per_sec: u64) {
        let _ = self.control.send(Control::SetSpeedLimit(bytes_per_sec));
    }
    pub fn set_threads(&self, threads: u32) {
        let _ = self.control.send(Control::SetThreads(threads));
    }
}

#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

pub(crate) struct EngineInner {
    pub(crate) cfg: EngineConfig,
    pub(crate) client: reqwest::Client,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Result<Self, EngineError> {
        let client = net::build_client(&cfg)?;
        Ok(Self { inner: Arc::new(EngineInner { cfg, client }) })
    }

    /// Ask the server about a link without downloading it (add dialog).
    pub async fn probe(&self, spec: &JobSpec) -> Result<ProbeInfo, EngineError> {
        let headers = net::build_headers(spec, &self.inner.cfg);
        let probed = net::probe_request(&self.inner, spec, &headers).await?;
        Ok(probed.info)
    }

    /// Start or resume a job. Must be called inside a tokio runtime.
    pub fn start(&self, spec: JobSpec, resume: Option<ResumeState>) -> JobHandle {
        let (events_tx, events) = mpsc::unbounded_channel();
        let (control, control_rx) = mpsc::unbounded_channel();
        let inner = self.inner.clone();
        tokio::spawn(async move {
            match spec.kind {
                SourceKind::Http => job::run(inner, spec, resume, events_tx, control_rx).await,
                SourceKind::Hls => hls::run(inner, spec, resume, events_tx, control_rx).await,
            }
        });
        JobHandle { events, control }
    }

    pub fn global_limit(&self) -> &Arc<SpeedLimiter> {
        &self.inner.cfg.global_limit
    }
}

/// Windows-safe file name: strips `<>:"/\|?*` and control characters, trailing dots
/// and spaces, and reserved device names.
pub fn sanitize_file_name(name: &str) -> String {
    names::sanitize_file_name(name)
}

/// File name from a Content-Disposition header (`filename*` first, then `filename`).
pub fn file_name_from_disposition(value: &str) -> Option<String> {
    names::file_name_from_disposition(value)
}

/// Last non-empty URL path segment, percent-decoded.
pub fn file_name_from_url(url: &str) -> Option<String> {
    names::file_name_from_url(url)
}
