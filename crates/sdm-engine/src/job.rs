use std::collections::HashSet;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::Response;
use reqwest::header::{HeaderMap, HeaderValue, IF_RANGE, RANGE};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::MissedTickBehavior;

use crate::files::{self, disk_err};
use crate::meter::SpeedMeter;
use crate::net::{self, Probed, backoff, net_err, retryable};
use crate::{
    Completed, Control, EngineError, EngineInner, Event, JobSpec, Part, ProbeInfo, Progress,
    ResumeState, SpeedLimiter, Validator,
};

const BUF: usize = 256 * 1024;
const MIB: u64 = 1024 * 1024;
/// End of a part whose length is not known.
const OPEN_END: u64 = u64::MAX - 1;

/// Why a job stopped before it could do any work.
pub(crate) enum Stop {
    Paused,
    Cancelled,
    Failed(EngineError),
}

/// What every kind of job shares: spec, event channel, controls and limits.
pub(crate) struct Shell {
    pub inner: Arc<EngineInner>,
    pub spec: JobSpec,
    pub tx: mpsc::UnboundedSender<Event>,
    pub ctrl: mpsc::UnboundedReceiver<Control>,
    pub headers: HeaderMap,
    pub limit: Arc<SpeedLimiter>,
    pub threads: u32,
}

impl Shell {
    pub(crate) fn new(
        inner: Arc<EngineInner>,
        spec: JobSpec,
        tx: mpsc::UnboundedSender<Event>,
        ctrl: mpsc::UnboundedReceiver<Control>,
    ) -> Self {
        let headers = net::build_headers(&spec, &inner.cfg);
        let limit = Arc::new(SpeedLimiter::new(spec.speed_limit));
        let threads = spec.threads.max(1);
        Self { inner, spec, tx, ctrl, headers, limit, threads }
    }

    pub(crate) fn emit(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    pub(crate) fn apply(&mut self, c: Control) -> Option<Stop> {
        match c {
            Control::Pause => return Some(Stop::Paused),
            Control::SetSpeedLimit(rate) => self.limit.set_rate(rate),
            Control::SetThreads(n) => self.threads = n.max(1),
        }
        None
    }

    /// Sleep, but wake for a pause or when the handle is dropped.
    pub(crate) async fn wait(&mut self, d: Duration) -> Option<Stop> {
        let sleep = tokio::time::sleep(d);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                _ = &mut sleep => return None,
                msg = self.ctrl.recv() => match msg {
                    None => return Some(Stop::Cancelled),
                    Some(c) => {
                        if let Some(stop) = self.apply(c) {
                            return Some(stop);
                        }
                    }
                },
            }
        }
    }

    async fn probe_retrying(&mut self) -> Result<Probed, Stop> {
        let mut failures = 0;
        loop {
            let err = match net::probe_request(&self.inner, &self.spec, &self.headers).await {
                Ok(p) => return Ok(p),
                Err(e) => e,
            };
            if !retryable(&err) || failures >= self.spec.max_retries {
                return Err(Stop::Failed(err));
            }
            failures += 1;
            self.emit(Event::Retrying { attempt: failures, error: err });
            if let Some(stop) = self.wait(backoff(failures)).await {
                return Err(stop);
            }
        }
    }
}

pub(crate) async fn run(
    inner: Arc<EngineInner>,
    spec: JobSpec,
    resume: Option<ResumeState>,
    tx: mpsc::UnboundedSender<Event>,
    ctrl: mpsc::UnboundedReceiver<Control>,
) {
    let events = tx.clone();
    let name_hint = spec.file_name.clone().unwrap_or_default();
    let sh = Shell::new(inner, spec, tx, ctrl);
    match Job::open(sh, resume).await {
        Ok(job) => job.drive().await,
        Err(Stop::Cancelled) => {}
        Err(Stop::Paused) => {
            let state = ResumeState { file_name: name_hint, ..ResumeState::default() };
            let _ = events.send(Event::Paused(state));
        }
        Err(Stop::Failed(error)) => {
            let state = ResumeState { file_name: name_hint, ..ResumeState::default() };
            let _ = events.send(Event::Failed { error, state });
        }
    }
}

/// One byte range shared between the coordinator and its worker. The end can shrink
/// while the worker runs, which is how a part is split.
pub(crate) struct PartCell {
    from: u64,
    span: Mutex<Span>,
}

struct Span {
    end: u64,
    current: u64,
    closed: bool,
}

impl PartCell {
    fn new(p: Part) -> Arc<Self> {
        Arc::new(Self {
            from: p.from,
            span: Mutex::new(Span { end: p.to, current: p.current, closed: false }),
        })
    }

    fn span(&self) -> MutexGuard<'_, Span> {
        self.span.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn snapshot(&self) -> Part {
        let s = self.span();
        Part { from: self.from, to: s.end, current: s.current }
    }

    fn done(&self) -> bool {
        let s = self.span();
        s.closed || s.current > s.end
    }

    fn remaining(&self) -> u64 {
        let s = self.span();
        if s.closed { 0 } else { (s.end + 1).saturating_sub(s.current) }
    }

    /// Claim room for up to `len` bytes; returns where to write and how many fit.
    fn reserve(&self, len: usize) -> (u64, usize) {
        let mut s = self.span();
        let room = (s.end + 1).saturating_sub(s.current);
        let n = (len as u64).min(room) as usize;
        let at = s.current;
        s.current += n as u64;
        (at, n)
    }

    fn reset(&self) {
        self.span().current = self.from;
    }

    /// The stream of an unknown-length part ended: what arrived is the whole part.
    fn close(&self) {
        let mut s = self.span();
        s.closed = true;
        s.end = s.current.saturating_sub(1);
    }

    fn split(&self, min_half: u64) -> Option<Part> {
        let mut s = self.span();
        if s.closed {
            return None;
        }
        let remaining = (s.end + 1).saturating_sub(s.current);
        let first_half = remaining / 2;
        if first_half < min_half || remaining - first_half < min_half {
            return None;
        }
        let mid = s.current + first_half;
        let old_end = s.end;
        s.end = mid - 1;
        Some(Part { from: mid, to: old_end, current: mid })
    }
}

struct Ctx {
    inner: Arc<EngineInner>,
    spec: JobSpec,
    headers: HeaderMap,
    file: Arc<File>,
    limit: Arc<SpeedLimiter>,
    validator: Validator,
    resumable: bool,
    total: Option<u64>,
    tx: mpsc::UnboundedSender<Event>,
}

enum WorkerEnd {
    Done,
    Stopped,
    Failed(EngineError),
}

enum Flow {
    Done,
    Stopped,
}

async fn stopped(rx: &mut watch::Receiver<bool>) {
    let _ = rx.wait_for(|v| *v).await;
}

fn check_status(ctx: &Ctx, resp: &Response, start: u64) -> Result<(), EngineError> {
    let status = resp.status().as_u16();
    if !ctx.resumable {
        return if resp.status().is_success() {
            Ok(())
        } else {
            Err(EngineError::Http { status })
        };
    }
    match status {
        206 => {
            let here = net::content_range(resp.headers()).map(|(from, _)| from);
            if here != Some(start) {
                return Err(EngineError::Changed);
            }
            let seen = net::header_text(resp.headers(), reqwest::header::ETAG);
            if let (Some(seen), Some(saved)) = (seen, &ctx.validator.etag)
                && &seen != saved
            {
                return Err(EngineError::Changed);
            }
            Ok(())
        }
        200 | 416 => Err(EngineError::Changed),
        _ => Err(EngineError::Http { status }),
    }
}

async fn request_part(ctx: &Ctx, cell: &PartCell) -> Result<Response, EngineError> {
    let span = cell.snapshot();
    let mut rb = net::get(&ctx.inner, &ctx.spec, &ctx.headers, &ctx.spec.url);
    if ctx.resumable {
        rb = rb.header(RANGE, format!("bytes={}-{}", span.current, span.to));
        let validator = ctx.validator.etag.as_ref().or(ctx.validator.last_modified.as_ref());
        if let Some(v) = validator
            && let Ok(v) = HeaderValue::from_str(v)
        {
            rb = rb.header(IF_RANGE, v);
        }
    }
    let resp = rb.send().await.map_err(|e| net_err(&e))?;
    check_status(ctx, &resp, span.current)?;
    Ok(resp)
}

async fn flush(ctx: &Arc<Ctx>, cell: &PartCell, buf: &mut Vec<u8>) -> Result<(), EngineError> {
    if buf.is_empty() {
        return Ok(());
    }
    let (at, n) = cell.reserve(buf.len());
    let mut data = std::mem::take(buf);
    data.truncate(n);
    if n == 0 {
        return Ok(());
    }
    let file = ctx.file.clone();
    tokio::task::spawn_blocking(move || files::write_all_at(&file, &data, at))
        .await
        .map_err(|e| EngineError::Disk(e.to_string()))?
        .map_err(disk_err)
}

async fn stream_part(
    ctx: &Arc<Ctx>,
    cell: &PartCell,
    resp: Response,
    stop: &mut watch::Receiver<bool>,
    progressed: &mut bool,
) -> Result<Flow, EngineError> {
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(BUF);
    loop {
        let item = tokio::select! {
            biased;
            _ = stopped(stop) => {
                flush(ctx, cell, &mut buf).await?;
                return Ok(Flow::Stopped);
            }
            item = stream.next() => item,
        };
        match item {
            None => {
                flush(ctx, cell, &mut buf).await?;
                if cell.done() {
                    return Ok(Flow::Done);
                }
                if ctx.total.is_none() {
                    cell.close();
                    return Ok(Flow::Done);
                }
                return Err(EngineError::Network("the connection closed before the data was complete".into()));
            }
            Some(Err(e)) => {
                let _ = flush(ctx, cell, &mut buf).await;
                return Err(net_err(&e));
            }
            Some(Ok(chunk)) => {
                *progressed = true;
                let n = chunk.len();
                tokio::select! {
                    biased;
                    _ = stopped(stop) => {
                        flush(ctx, cell, &mut buf).await?;
                        return Ok(Flow::Stopped);
                    }
                    _ = async {
                        ctx.limit.acquire(n).await;
                        ctx.inner.cfg.global_limit.acquire(n).await;
                    } => {}
                }
                buf.extend_from_slice(&chunk);
                let reaches_end = {
                    let s = cell.span();
                    s.current.saturating_add(buf.len() as u64) > s.end
                };
                if buf.len() >= BUF || reaches_end {
                    flush(ctx, cell, &mut buf).await?;
                    if cell.done() {
                        return Ok(Flow::Done);
                    }
                }
            }
        }
    }
}

async fn worker(
    ctx: Arc<Ctx>,
    cell: Arc<PartCell>,
    mut first: Option<Response>,
    mut stop: watch::Receiver<bool>,
) -> WorkerEnd {
    let mut failures = 0u32;
    loop {
        if *stop.borrow() {
            return WorkerEnd::Stopped;
        }
        if cell.done() {
            return WorkerEnd::Done;
        }
        let mut progressed = false;
        let result = match first.take() {
            Some(resp) => stream_part(&ctx, &cell, resp, &mut stop, &mut progressed).await,
            None => match request_part(&ctx, &cell).await {
                Ok(resp) => stream_part(&ctx, &cell, resp, &mut stop, &mut progressed).await,
                Err(e) => Err(e),
            },
        };
        let err = match result {
            Ok(Flow::Done) => return WorkerEnd::Done,
            Ok(Flow::Stopped) => return WorkerEnd::Stopped,
            Err(e) => e,
        };
        if !retryable(&err) {
            return WorkerEnd::Failed(err);
        }
        if progressed {
            failures = 0;
        }
        failures += 1;
        if failures > ctx.spec.max_retries {
            return WorkerEnd::Failed(err);
        }
        let _ = ctx.tx.send(Event::Retrying { attempt: failures, error: err });
        if !ctx.resumable {
            cell.reset();
        }
        tokio::select! {
            _ = stopped(&mut stop) => return WorkerEnd::Stopped,
            _ = tokio::time::sleep(backoff(failures)) => {}
        }
    }
}

enum End {
    Completed,
    Paused,
    Cancelled,
    Changed,
    Failed(EngineError),
}

enum Reason {
    Pause,
    Cancel,
    Changed,
    Failed(EngineError),
}

struct Job {
    sh: Shell,
    file_name: String,
    working_path: PathBuf,
    file: Arc<File>,
    total: Option<u64>,
    resumable: bool,
    validator: Validator,
    cells: Vec<Arc<PartCell>>,
    first: Option<Response>,
    changed_once: bool,
    speed: u64,
}

fn plan_parts(total: Option<u64>, resumable: bool, threads: u32, min_part: u64) -> Vec<Part> {
    let whole = |to| vec![Part { from: 0, to, current: 0 }];
    match (total, resumable) {
        (Some(0), _) => Vec::new(),
        (Some(t), true) => {
            let by_size = (t / min_part.max(1)).max(1);
            let n = u64::from(threads.max(1)).min(by_size);
            let chunk = t / n;
            (0..n)
                .map(|i| {
                    let from = i * chunk;
                    let to = if i == n - 1 { t - 1 } else { from + chunk - 1 };
                    Part { from, to, current: from }
                })
                .collect()
        }
        (Some(t), false) => whole(t - 1),
        (None, _) => whole(OPEN_END),
    }
}

impl Job {
    async fn open(sh: Shell, resume: Option<ResumeState>) -> Result<Job, Stop> {
        match resume {
            Some(st) if st.resumable && st.total.is_some() && !st.parts.is_empty() => {
                Self::resume(sh, st).await
            }
            Some(st) if !st.working_path.as_os_str().is_empty() => {
                Self::fresh(sh, Some((st.working_path, st.file_name))).await
            }
            _ => Self::fresh(sh, None).await,
        }
    }

    async fn resume(sh: Shell, st: ResumeState) -> Result<Job, Stop> {
        let total = st.total.unwrap_or(0);
        let furthest = st.parts.iter().map(|p| p.current).max().unwrap_or(0).min(total);
        let usable = files::open_existing(&st.working_path).ok().and_then(|f| {
            let len = f.metadata().ok()?.len();
            (len >= furthest).then_some((f, len))
        });
        let Some((file, len)) = usable else {
            return Self::fresh(sh, Some((st.working_path, st.file_name))).await;
        };
        if len < total {
            file.set_len(total).map_err(|e| Stop::Failed(disk_err(e)))?;
        }
        Ok(Job {
            sh,
            file_name: st.file_name,
            working_path: st.working_path,
            file: Arc::new(file),
            total: st.total,
            resumable: true,
            validator: st.validator,
            cells: st.parts.into_iter().map(PartCell::new).collect(),
            first: None,
            changed_once: false,
            speed: 0,
        })
    }

    async fn fresh(mut sh: Shell, reuse: Option<(PathBuf, String)>) -> Result<Job, Stop> {
        let probed = sh.probe_retrying().await?;
        if probed.info.needs_auth {
            return Err(Stop::Failed(EngineError::Http { status: 401 }));
        }
        let claimed = match reuse {
            Some((path, name)) => files::reclaim(&path, &name),
            None => files::claim(&sh.spec, &probed.info.file_name),
        }
        .map_err(Stop::Failed)?;
        sh.emit(Event::Probed(probed.info.clone()));
        let mut job = Job {
            sh,
            file_name: claimed.file_name,
            working_path: claimed.working_path,
            file: Arc::new(claimed.file),
            total: None,
            resumable: false,
            validator: Validator::default(),
            cells: Vec::new(),
            first: None,
            changed_once: false,
            speed: 0,
        };
        job.apply_probe(&probed.info, probed.response).map_err(Stop::Failed)?;
        Ok(job)
    }

    fn apply_probe(&mut self, info: &ProbeInfo, resp: Response) -> Result<(), EngineError> {
        if self.sh.spec.sparse {
            files::mark_sparse(&self.file);
        }
        if let Some(t) = info.total.filter(|t| *t > 0) {
            self.file.set_len(t).map_err(disk_err)?;
        }
        self.total = info.total;
        self.resumable = info.resumable;
        self.validator = info.validator.clone();
        let parts = plan_parts(info.total, info.resumable, self.sh.threads, self.sh.spec.min_part_size);
        self.cells = parts.into_iter().map(PartCell::new).collect();
        self.first = Some(resp);
        Ok(())
    }

    fn downloaded(&self) -> u64 {
        self.cells.iter().map(|c| {
            let p = c.snapshot();
            p.current.saturating_sub(p.from)
        }).sum()
    }

    fn parts(&self) -> Vec<Part> {
        self.cells.iter().map(|c| c.snapshot()).collect()
    }

    fn state(&self) -> ResumeState {
        ResumeState {
            file_name: self.file_name.clone(),
            working_path: self.working_path.clone(),
            total: self.total,
            resumable: self.resumable,
            validator: self.validator.clone(),
            parts: self.parts(),
            hls_next_segment: None,
        }
    }

    fn progress(&self, active: usize) -> Progress {
        Progress {
            downloaded: self.downloaded(),
            total: self.total,
            speed: self.speed,
            active_connections: active as u32,
            parts: self.parts(),
        }
    }

    fn try_split(&mut self, active: &HashSet<u64>) -> Option<Arc<PartCell>> {
        if !self.sh.spec.dynamic_parts || !self.resumable {
            return None;
        }
        let min_half = self.sh.spec.min_part_size.max(MIB);
        let (idx, _) = self
            .cells
            .iter()
            .enumerate()
            .filter(|(_, c)| active.contains(&c.from))
            .max_by_key(|(_, c)| c.remaining())?;
        let part = self.cells[idx].split(min_half)?;
        let cell = PartCell::new(part);
        self.cells.insert(idx + 1, cell.clone());
        Some(cell)
    }

    fn fill(
        &mut self,
        ctx: &Arc<Ctx>,
        set: &mut JoinSet<(u64, WorkerEnd)>,
        active: &mut HashSet<u64>,
        first: &mut Option<Response>,
        stop: &watch::Receiver<bool>,
        target: u32,
    ) {
        let mut split = false;
        while (active.len() as u32) < target {
            let pending = self.cells.iter().find(|c| !c.done() && !active.contains(&c.from)).cloned();
            let cell = match pending {
                Some(c) => c,
                None => match self.try_split(active) {
                    Some(c) => {
                        split = true;
                        c
                    }
                    None => break,
                },
            };
            active.insert(cell.from);
            let resp = if cell.from == 0 { first.take() } else { None };
            let (ctx, stop, from) = (ctx.clone(), stop.clone(), cell.from);
            set.spawn(async move { (from, worker(ctx, cell, resp, stop).await) });
        }
        if split {
            self.sh.emit(Event::Progress(self.progress(active.len())));
        }
    }

    async fn session(&mut self) -> End {
        let (stop_tx, stop_rx) = watch::channel(false);
        let ctx = Arc::new(Ctx {
            inner: self.sh.inner.clone(),
            spec: self.sh.spec.clone(),
            headers: self.sh.headers.clone(),
            file: self.file.clone(),
            limit: self.sh.limit.clone(),
            validator: self.validator.clone(),
            resumable: self.resumable,
            total: self.total,
            tx: self.sh.tx.clone(),
        });
        let mut set: JoinSet<(u64, WorkerEnd)> = JoinSet::new();
        let mut active: HashSet<u64> = HashSet::new();
        let mut first = self.first.take();
        let mut target = self.sh.threads;
        let mut meter = SpeedMeter::new(self.downloaded());
        let mut tick = tokio::time::interval(Duration::from_millis(500));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        tick.tick().await;
        let mut reason: Option<Reason> = None;

        self.fill(&ctx, &mut set, &mut active, &mut first, &stop_rx, target);
        while reason.is_none() {
            if active.is_empty() && self.cells.iter().all(|c| c.done()) {
                break;
            }
            tokio::select! {
                Some(joined) = set.join_next() => {
                    match joined {
                        Ok((from, end)) => {
                            active.remove(&from);
                            match end {
                                WorkerEnd::Done | WorkerEnd::Stopped => {}
                                WorkerEnd::Failed(EngineError::Changed) => reason = Some(Reason::Changed),
                                WorkerEnd::Failed(e) => reason = Some(Reason::Failed(e)),
                            }
                        }
                        Err(e) => reason = Some(Reason::Failed(EngineError::Other(format!("a download task crashed: {e}")))),
                    }
                    if reason.is_none() {
                        self.fill(&ctx, &mut set, &mut active, &mut first, &stop_rx, target);
                    }
                }
                msg = self.sh.ctrl.recv() => match msg {
                    None => reason = Some(Reason::Cancel),
                    Some(Control::Pause) => reason = Some(Reason::Pause),
                    Some(Control::SetSpeedLimit(rate)) => self.sh.limit.set_rate(rate),
                    Some(Control::SetThreads(n)) => {
                        target = n.max(1);
                        self.sh.threads = target;
                        self.fill(&ctx, &mut set, &mut active, &mut first, &stop_rx, target);
                    }
                },
                _ = tick.tick() => {
                    self.speed = meter.sample(self.downloaded());
                    self.sh.emit(Event::Progress(self.progress(active.len())));
                }
            }
        }

        let Some(reason) = reason else {
            return End::Completed;
        };
        let _ = stop_tx.send(true);
        while set.join_next().await.is_some() {}
        match reason {
            Reason::Pause => End::Paused,
            Reason::Cancel => End::Cancelled,
            Reason::Changed => End::Changed,
            Reason::Failed(e) => End::Failed(e),
        }
    }

    async fn restart(&mut self) -> Result<(), Stop> {
        let probed = self.sh.probe_retrying().await?;
        if probed.info.needs_auth {
            return Err(Stop::Failed(EngineError::Http { status: 401 }));
        }
        let claimed = files::reclaim(&self.working_path, &self.file_name).map_err(Stop::Failed)?;
        self.file = Arc::new(claimed.file);
        self.apply_probe(&probed.info, probed.response).map_err(Stop::Failed)
    }

    async fn sync(&self) {
        let file = self.file.clone();
        let _ = tokio::task::spawn_blocking(move || file.sync_data()).await;
    }

    async fn drive(mut self) {
        let mut end = self.session().await;
        loop {
            match end {
                End::Completed => {
                    if let Err(error) = self.complete().await {
                        let state = self.state();
                        self.sh.emit(Event::Failed { error, state });
                    }
                    return;
                }
                End::Paused => {
                    self.sync().await;
                    let state = self.state();
                    self.sh.emit(Event::Paused(state));
                    return;
                }
                End::Cancelled => return,
                End::Failed(error) => {
                    self.sync().await;
                    let state = self.state();
                    self.sh.emit(Event::Failed { error, state });
                    return;
                }
                End::Changed => {
                    if self.changed_once {
                        end = End::Failed(EngineError::Changed);
                        continue;
                    }
                    self.changed_once = true;
                    self.sh.emit(Event::Retrying { attempt: 1, error: EngineError::Changed });
                    end = match self.restart().await {
                        Ok(()) => self.session().await,
                        Err(Stop::Paused) => End::Paused,
                        Err(Stop::Cancelled) => End::Cancelled,
                        Err(Stop::Failed(e)) => End::Failed(e),
                    };
                }
            }
        }
    }

    async fn complete(&mut self) -> Result<(), EngineError> {
        let downloaded = self.downloaded();
        let file = self.file.clone();
        let unknown_total = self.total.is_none();
        let mtime = if self.sh.spec.apply_server_mtime {
            self.validator.last_modified.as_deref().and_then(files::parse_http_date)
        } else {
            None
        };
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            if unknown_total {
                file.set_len(downloaded)?;
            }
            let _ = file.sync_data();
            if let Some(t) = mtime {
                let _ = file.set_modified(t);
            }
            Ok(())
        })
        .await
        .map_err(|e| EngineError::Disk(e.to_string()))?
        .map_err(disk_err)?;

        let (path, file_name) =
            files::finalize(&self.working_path, &self.file_name, self.sh.spec.on_exists)?;
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(downloaded);
        self.sh.emit(Event::Progress(self.progress(0)));
        self.sh.emit(Event::Completed(Completed { path, file_name, size }));
        Ok(())
    }
}
