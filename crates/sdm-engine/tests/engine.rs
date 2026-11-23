use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use axum::Router;
use axum::body::Body;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use bytes::Bytes;
use sdm_engine::{
    Engine, EngineConfig, EngineError, Event, JobHandle, JobSpec, OnExists, ProxyConfig,
    ResumeState, SourceKind,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const CHUNK: u64 = 64 * 1024;
const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;

fn byte(version: u8, i: u64) -> u8 {
    (i.wrapping_mul(31).wrapping_add(i / 251).wrapping_add(u64::from(version) * 97) % 256) as u8
}

fn data(version: u8, from: u64, to: u64) -> Vec<u8> {
    (from..to).map(|i| byte(version, i)).collect()
}

fn sha(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn expected_hash(version: u8, size: u64) -> Vec<u8> {
    sha(&data(version, 0, size))
}

#[derive(Default)]
struct Fixture {
    hits: AtomicU64,
    dropped: AtomicBool,
}

#[derive(Deserialize, Default)]
struct Opts {
    range: Option<u8>,
    slow_from: Option<u64>,
    delay: Option<u64>,
    drop_at: Option<u64>,
    change_after: Option<u64>,
}

struct Extra {
    headers: Vec<(&'static str, String)>,
}

fn parse_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let spec = value.strip_prefix("bytes=")?;
    let (a, b) = spec.split_once('-')?;
    let from: u64 = a.parse().ok()?;
    let to = if b.is_empty() { size.checked_sub(1)? } else { b.parse::<u64>().ok()?.min(size.checked_sub(1)?) };
    (from <= to).then_some((from, to))
}

fn respond(fx: &Fixture, size: u64, opts: &Opts, req: &HeaderMap, extra: Extra) -> Response {
    let hit = fx.hits.fetch_add(1, Ordering::SeqCst);
    let version: u8 = match opts.change_after {
        Some(n) if hit >= n => 2,
        _ => 1,
    };
    let etag = format!("\"v{version}\"");
    let ranges_ok = opts.range.unwrap_or(1) == 1;
    let mut range = if ranges_ok {
        req.get(header::RANGE).and_then(|v| v.to_str().ok()).and_then(|v| parse_range(v, size))
    } else {
        None
    };
    if let Some(ir) = req.get(header::IF_RANGE).and_then(|v| v.to_str().ok())
        && ir != etag
    {
        range = None;
    }
    let (from, to) = range.unwrap_or((0, size.saturating_sub(1)));
    let len = if size == 0 { 0 } else { to - from + 1 };
    let dropper = opts.drop_at.is_some() && !fx.dropped.swap(true, Ordering::SeqCst);
    let drop_at = opts.drop_at.unwrap_or(u64::MAX);
    let slow_from = opts.slow_from;
    let delay = Duration::from_millis(opts.delay.unwrap_or(20));

    let body = futures_util::stream::unfold((from, 0u64), move |(pos, sent)| async move {
        if sent >= len {
            return None;
        }
        if dropper && sent >= drop_at {
            return Some((Err(io::Error::other("dropped on purpose")), (pos, len)));
        }
        if slow_from.is_some_and(|s| pos >= s) {
            tokio::time::sleep(delay).await;
        }
        let mut n = CHUNK.min(len - sent);
        if dropper {
            n = n.min(drop_at.saturating_sub(sent).max(1));
        }
        let chunk = Bytes::from(data(version, pos, pos + n));
        Some((Ok::<_, io::Error>(chunk), (pos + n, sent + n)))
    });

    let mut resp = Response::builder()
        .status(if range.is_some() { StatusCode::PARTIAL_CONTENT } else { StatusCode::OK })
        .header(header::CONTENT_LENGTH, len)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::ETAG, etag)
        .header(header::LAST_MODIFIED, "Sun, 06 Nov 1994 08:49:37 GMT");
    if ranges_ok {
        resp = resp.header(header::ACCEPT_RANGES, "bytes");
    }
    if range.is_some() {
        resp = resp.header(header::CONTENT_RANGE, format!("bytes {from}-{to}/{size}"));
    }
    for (k, v) in extra.headers {
        resp = resp.header(k, v);
    }
    resp.body(Body::from_stream(body)).unwrap()
}

async fn file(
    State(fx): State<Arc<Fixture>>,
    UrlPath(size): UrlPath<u64>,
    Query(opts): Query<Opts>,
    req: HeaderMap,
) -> Response {
    respond(&fx, size, &opts, &req, Extra { headers: vec![] })
}

async fn named(State(fx): State<Arc<Fixture>>, UrlPath(_name): UrlPath<String>, req: HeaderMap) -> Response {
    respond(&fx, 300_000, &Opts::default(), &req, Extra { headers: vec![] })
}

async fn disposition(State(fx): State<Arc<Fixture>>, UrlPath(kind): UrlPath<String>, req: HeaderMap) -> Response {
    let value = match kind.as_str() {
        "plain" => "attachment; filename=plain.bin".to_string(),
        "ext" => "attachment; filename=\"fallback.bin\"; filename*=UTF-8''na%C3%AFve%20%E2%82%AC.bin".to_string(),
        _ => "attachment; filename=\"my file name.bin\"".to_string(),
    };
    respond(&fx, 100_000, &Opts::default(), &req, Extra { headers: vec![("content-disposition", value)] })
}

async fn redirect(UrlPath(n): UrlPath<u32>) -> Response {
    let to = if n == 0 { "/named/final.bin".to_string() } else { format!("/redirect/{}", n - 1) };
    Response::builder().status(StatusCode::FOUND).header(header::LOCATION, to).body(Body::empty()).unwrap()
}

async fn endless_redirect() -> Response {
    Response::builder().status(StatusCode::FOUND).header(header::LOCATION, "/loop").body(Body::empty()).unwrap()
}

async fn auth() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "Basic realm=\"x\"")
        .body(Body::empty())
        .unwrap()
}

async fn html() -> Response {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], "<html>hello</html>").into_response()
}

async fn typed() -> Response {
    ([(header::CONTENT_TYPE, "application/zip")], vec![7u8; 2000]).into_response()
}

async fn missing() -> Response {
    StatusCode::NOT_FOUND.into_response()
}

async fn echo(req: HeaderMap) -> Response {
    let mut out = String::new();
    for (k, v) in &req {
        out.push_str(&format!("{}: {}\n", k.as_str(), v.to_str().unwrap_or("?")));
    }
    ([(header::CONTENT_TYPE, "text/plain")], out).into_response()
}

fn seg(n: usize) -> Vec<u8> {
    data(n as u8 + 1, 0, 30_000 + n as u64 * 777)
}

async fn hls_file(UrlPath(name): UrlPath<String>) -> Response {
    let text = |t: &str| ([(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")], t.to_string()).into_response();
    match name.as_str() {
        "master.m3u8" => text("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1000\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=5000\nhigh.m3u8\n"),
        "high.m3u8" => text("#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4,\ns0.ts\n#EXTINF:4,\ns1.ts\n#EXTINF:4,\ns2.ts\n#EXT-X-ENDLIST\n"),
        "low.m3u8" => text("#EXTM3U\n#EXTINF:4,\nwrong.ts\n#EXT-X-ENDLIST\n"),
        "enc.m3u8" => text("#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:4,\ns0.ts\n#EXT-X-ENDLIST\n"),
        "s0.ts" => (StatusCode::OK, seg(0)).into_response(),
        "s1.ts" => (StatusCode::OK, seg(1)).into_response(),
        "s2.ts" => (StatusCode::OK, seg(2)).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve() -> (String, Arc<Fixture>) {
    let fx = Arc::new(Fixture::default());
    let app = Router::new()
        .route("/f/{size}", get(file))
        .route("/named/{name}", get(named))
        .route("/disp/{kind}", get(disposition))
        .route("/redirect/{n}", get(redirect))
        .route("/loop", get(endless_redirect))
        .route("/auth", get(auth))
        .route("/html", get(html))
        .route("/typed", get(typed))
        .route("/missing", get(missing))
        .route("/echo", get(echo))
        .route("/hls/{name}", get(hls_file))
        .with_state(fx.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), fx)
}

fn engine() -> Engine {
    engine_with(|_| {})
}

fn engine_with(tweak: impl FnOnce(&mut EngineConfig)) -> Engine {
    let mut cfg = EngineConfig { proxy: ProxyConfig::Direct, ..EngineConfig::default() };
    tweak(&mut cfg);
    Engine::new(cfg).unwrap()
}

fn spec(url: String, dir: &Path) -> JobSpec {
    let mut s = JobSpec::new(url, dir);
    s.min_part_size = 256 * KIB;
    s
}

#[derive(Debug)]
struct Run {
    events: Vec<Event>,
}

impl Run {
    fn last(&self) -> &Event {
        self.events.last().unwrap()
    }

    fn completed(&self) -> &sdm_engine::Completed {
        match self.last() {
            Event::Completed(c) => c,
            other => panic!("expected Completed, got {other:?}"),
        }
    }

    fn retries(&self) -> Vec<&EngineError> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Retrying { error, .. } => Some(error),
                _ => None,
            })
            .collect()
    }

    fn max_parts(&self) -> usize {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Progress(p) => Some(p.parts.len()),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    fn probed(&self) -> &sdm_engine::ProbeInfo {
        self.events
            .iter()
            .find_map(|e| match e {
                Event::Probed(p) => Some(p),
                _ => None,
            })
            .expect("no Probed event")
    }
}

async fn finish(mut h: JobHandle) -> Run {
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let ev = tokio::time::timeout(left, h.events.recv())
            .await
            .expect("the job took longer than 60 s")
            .expect("the job ended without a final event");
        let end = matches!(ev, Event::Completed(_) | Event::Failed { .. } | Event::Paused(_));
        events.push(ev);
        if end {
            return Run { events };
        }
    }
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

macro_rules! rt {
    ($(#[$m:meta])* async fn $name:ident() $body:block) => {
        $(#[$m])*
        #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
        async fn $name() $body
    };
}

rt! {
async fn multi_part_download_matches_hash() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/4194304"), dir.path());
    s.threads = 4;
    let run = finish(engine().start(s, None)).await;
    let done = run.completed();
    assert!(matches!(run.events[0], Event::Probed(_)));
    assert!(run.probed().resumable);
    assert_eq!(run.probed().total, Some(4 * MIB));
    assert_eq!(done.size, 4 * MIB);
    assert_eq!(done.file_name, "4194304");
    assert_eq!(sha(&read(&done.path)), expected_hash(1, 4 * MIB));
}
}

rt! {
async fn probe_reports_the_link() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let info = engine().probe(&spec(format!("{base}/f/1000"), dir.path())).await.unwrap();
    assert!(info.resumable && !info.is_web_page && !info.needs_auth);
    assert_eq!(info.total, Some(1000));
    assert_eq!(info.validator.etag.as_deref(), Some("\"v1\""));
    assert!(info.validator.last_modified.is_some());
}
}

rt! {
async fn dynamic_split_adds_parts() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/8388608?slow_from=4194304&delay=20"), dir.path());
    s.threads = 2;
    let run = finish(engine().start(s, None)).await;
    let done = run.completed();
    assert!(run.max_parts() > 2, "parts seen: {}", run.max_parts());
    assert_eq!(sha(&read(&done.path)), expected_hash(1, 8 * MIB));
}
}

rt! {
async fn no_split_when_dynamic_parts_off() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/8388608?slow_from=4194304&delay=10"), dir.path());
    s.threads = 2;
    s.dynamic_parts = false;
    let run = finish(engine().start(s, None)).await;
    assert!(run.max_parts() <= 2);
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, 8 * MIB));
}
}

rt! {
async fn pause_then_resume_completes() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let url = format!("{base}/f/8388608?slow_from=0&delay=40");
    let mut s = spec(url, dir.path());
    s.threads = 4;
    s.temp_suffix = Some(".sdm-part".into());
    let eng = engine();
    let h = eng.start(s.clone(), None);
    tokio::time::sleep(Duration::from_millis(500)).await;
    h.pause();
    let paused = finish(h).await;
    let Event::Paused(state) = paused.last() else { panic!("expected Paused, got {:?}", paused.last()) };
    assert!(state.resumable);
    assert!(state.downloaded() > 0 && state.downloaded() < 8 * MIB, "downloaded {}", state.downloaded());
    assert!(state.working_path.exists());

    let json_free = state.clone();
    let run = finish(eng.start(s, Some(json_free))).await;
    let done = run.completed();
    assert_eq!(sha(&read(&done.path)), expected_hash(1, 8 * MIB));
    assert!(!state.working_path.exists());
}
}

rt! {
async fn non_resumable_completes() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/2097152?range=0"), dir.path());
    s.threads = 4;
    let run = finish(engine().start(s, None)).await;
    assert!(!run.probed().resumable);
    assert_eq!(run.probed().total, Some(2 * MIB));
    assert_eq!(run.max_parts().max(1), 1);
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, 2 * MIB));
}
}

rt! {
async fn non_resumable_restarts_after_a_drop() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let s = spec(format!("{base}/f/1048576?range=0&drop_at=300000"), dir.path());
    let run = finish(engine().start(s, None)).await;
    assert_eq!(run.retries().len(), 1);
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, MIB));
}
}

rt! {
async fn dropped_connection_is_retried_and_resumed() {
    let (base, fx) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/4194304?drop_at=524288"), dir.path());
    s.threads = 4;
    let run = finish(engine().start(s, None)).await;
    let retries = run.retries();
    assert!(!retries.is_empty());
    assert!(matches!(retries[0], EngineError::Network(_)));
    assert!(fx.dropped.load(Ordering::SeqCst));
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, 4 * MIB));
}
}

rt! {
async fn changed_etag_restarts_from_zero() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/4194304?change_after=2"), dir.path());
    s.threads = 4;
    let run = finish(engine().start(s, None)).await;
    let changed = run.retries().into_iter().filter(|e| **e == EngineError::Changed).count();
    assert_eq!(changed, 1);
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(2, 4 * MIB));
}
}

rt! {
async fn resume_with_changed_file_restarts() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let url = format!("{base}/f/2097152");
    let mut s = spec(url, dir.path());
    s.threads = 2;
    s.temp_suffix = Some(".part".into());
    let eng = engine();
    let working = dir.path().join("2097152.part");
    std::fs::write(&working, vec![0xEE; 2 * MIB as usize]).unwrap();
    let mut state = ResumeState {
        file_name: "2097152".into(),
        working_path: working,
        total: Some(2 * MIB),
        resumable: true,
        ..ResumeState::default()
    };
    state.validator.etag = Some("\"old\"".into());
    state.parts = vec![
        sdm_engine::Part { from: 0, to: MIB - 1, current: MIB },
        sdm_engine::Part { from: MIB, to: 2 * MIB - 1, current: MIB + 100 },
    ];
    let run = finish(eng.start(s, Some(state))).await;
    assert_eq!(run.retries(), vec![&EngineError::Changed]);
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, 2 * MIB));
}
}

rt! {
async fn needs_auth_and_errors() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let eng = engine();
    let info = eng.probe(&spec(format!("{base}/auth"), dir.path())).await.unwrap();
    assert!(info.needs_auth);
    let run = finish(eng.start(spec(format!("{base}/auth"), dir.path()), None)).await;
    assert!(matches!(run.last(), Event::Failed { error: EngineError::Http { status: 401 }, .. }));
    let run = finish(eng.start(spec(format!("{base}/missing"), dir.path()), None)).await;
    assert!(matches!(run.last(), Event::Failed { error: EngineError::Http { status: 404 }, .. }));
    assert!(run.retries().is_empty());
}
}

rt! {
async fn web_page_is_flagged() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let info = engine().probe(&spec(format!("{base}/html"), dir.path())).await.unwrap();
    assert!(info.is_web_page);
    assert!(!info.resumable);
}
}

rt! {
async fn file_names_come_from_the_server() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let eng = engine();
    for (kind, want) in [("plain", "plain.bin"), ("ext", "na\u{ef}ve \u{20ac}.bin"), ("quoted", "my file name.bin")] {
        let info = eng.probe(&spec(format!("{base}/disp/{kind}"), dir.path())).await.unwrap();
        assert_eq!(info.file_name, want, "kind {kind}");
    }
    let mut s = spec(format!("{base}/disp/quoted"), dir.path());
    s.file_name = Some("mine?.bin".into());
    assert_eq!(eng.probe(&s).await.unwrap().file_name, "mine.bin");
    let info = eng.probe(&spec(format!("{base}/typed"), dir.path())).await.unwrap();
    assert_eq!(info.file_name, "typed.zip");
}
}

rt! {
async fn redirect_chain_is_followed() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let eng = engine();
    let run = finish(eng.start(spec(format!("{base}/redirect/3"), dir.path()), None)).await;
    let done = run.completed();
    assert_eq!(done.file_name, "final.bin");
    assert!(run.probed().final_url.ends_with("/named/final.bin"));
    assert_eq!(sha(&read(&done.path)), expected_hash(1, 300_000));

    let err = eng.probe(&spec(format!("{base}/loop"), dir.path())).await.unwrap_err();
    assert!(matches!(err, EngineError::Network(_)), "{err:?}");
}
}

rt! {
async fn request_headers_are_cleaned() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/echo"), dir.path());
    s.headers = vec![
        ("Host".into(), "evil.test".into()),
        ("Range".into(), "bytes=5-9".into()),
        ("If-None-Match".into(), "x".into()),
        ("Accept-Encoding".into(), "gzip".into()),
        ("Sec-Fetch-Mode".into(), "navigate".into()),
        ("X-Mine".into(), "yes".into()),
        ("Cookie".into(), "a=b".into()),
    ];
    s.referrer = Some("http://ref.test/page".into());
    s.basic_auth = Some(("user".into(), "pw".into()));
    let run = finish(engine().start(s, None)).await;
    let text = String::from_utf8(read(&run.completed().path)).unwrap().to_lowercase();
    assert!(!text.contains("evil.test"));
    assert!(!text.contains("bytes=5-9"));
    assert!(!text.contains("if-none-match"));
    assert!(!text.contains("gzip"));
    assert!(!text.contains("sec-fetch"));
    assert!(text.contains("x-mine: yes"));
    assert!(text.contains("cookie: a=b"));
    assert!(text.contains("referer: http://ref.test/page"));
    assert!(text.contains("accept-encoding: identity"));
    assert!(text.contains("user-agent: mozilla/5.0"));
    assert!(text.contains("authorization: basic dXNlcjpwdw==".to_lowercase().as_str()));
}
}

rt! {
async fn user_agent_override_wins() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/echo"), dir.path());
    s.user_agent = Some("SdmTest/1".into());
    let run = finish(engine().start(s, None)).await;
    let text = String::from_utf8(read(&run.completed().path)).unwrap();
    assert!(text.contains("user-agent: SdmTest/1"));
}
}

rt! {
async fn job_speed_limit_is_respected() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/1048576"), dir.path());
    s.speed_limit = 512 * KIB;
    let t = Instant::now();
    let run = finish(engine().start(s, None)).await;
    let took = t.elapsed();
    assert!(took >= Duration::from_millis(1500), "took {took:?}");
    assert!(took < Duration::from_secs(8), "took {took:?}");
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, MIB));
}
}

rt! {
async fn global_speed_limit_is_respected() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let eng = engine_with(|c| c.global_limit.set_rate(512 * KIB));
    let t = Instant::now();
    let run = finish(eng.start(spec(format!("{base}/f/1048576"), dir.path()), None)).await;
    assert!(t.elapsed() >= Duration::from_millis(1500), "took {:?}", t.elapsed());
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, MIB));
}
}

rt! {
async fn speed_limit_can_change_live() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/1048576"), dir.path());
    s.speed_limit = 32 * KIB;
    let h = engine().start(s, None);
    tokio::time::sleep(Duration::from_millis(400)).await;
    let t = Instant::now();
    h.set_speed_limit(0);
    let run = finish(h).await;
    assert!(t.elapsed() < Duration::from_secs(5), "took {:?}", t.elapsed());
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, MIB));
}
}

rt! {
async fn existing_names_are_numbered() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plain.bin"), b"old").unwrap();
    let eng = engine();
    let url = format!("{base}/disp/plain");
    let first = finish(eng.start(spec(url.clone(), dir.path()), None)).await;
    assert_eq!(first.completed().file_name, "plain_1.bin");
    let second = finish(eng.start(spec(url.clone(), dir.path()), None)).await;
    assert_eq!(second.completed().file_name, "plain_2.bin");
    assert_eq!(read(&dir.path().join("plain.bin")), b"old");
    assert_eq!(sha(&read(&second.completed().path)), expected_hash(1, 100_000));
}
}

rt! {
async fn temp_name_counts_as_taken() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plain.bin.sdm-part"), b"someone else").unwrap();
    let mut s = spec(format!("{base}/disp/plain"), dir.path());
    s.temp_suffix = Some(".sdm-part".into());
    let run = finish(engine().start(s, None)).await;
    assert_eq!(run.completed().file_name, "plain_1.bin");
    assert_eq!(read(&dir.path().join("plain.bin.sdm-part")), b"someone else");
}
}

rt! {
async fn overwrite_and_fail_policies() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("plain.bin"), b"old").unwrap();
    let eng = engine();
    let mut s = spec(format!("{base}/disp/plain"), dir.path());
    s.on_exists = OnExists::Fail;
    let run = finish(eng.start(s.clone(), None)).await;
    assert!(matches!(run.last(), Event::Failed { error: EngineError::Exists, .. }));
    assert_eq!(read(&dir.path().join("plain.bin")), b"old");
    s.on_exists = OnExists::Overwrite;
    let run = finish(eng.start(s, None)).await;
    assert_eq!(run.completed().file_name, "plain.bin");
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, 100_000));
}
}

rt! {
async fn temp_suffix_is_renamed_on_completion() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/2097152?slow_from=0&delay=30"), dir.path());
    s.threads = 2;
    s.temp_suffix = Some(".sdm-part".into());
    let mut h = engine().start(s, None);
    let temp = dir.path().join("2097152.sdm-part");
    let final_path = dir.path().join("2097152");
    let mut checked = false;
    let done = loop {
        match h.events.recv().await.unwrap() {
            Event::Progress(_) if !checked => {
                assert!(temp.exists());
                assert!(!final_path.exists());
                checked = true;
            }
            Event::Completed(c) => break c,
            Event::Failed { error, .. } => panic!("{error}"),
            _ => {}
        }
    };
    assert!(checked, "never saw progress before completion");
    assert_eq!(done.path, final_path);
    assert!(!temp.exists());
    assert_eq!(sha(&read(&final_path)), expected_hash(1, 2 * MIB));
}
}

rt! {
async fn creates_missing_folder_and_applies_server_time() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let nested: PathBuf = dir.path().join("a").join("b");
    let mut s = spec(format!("{base}/disp/plain"), &nested);
    s.apply_server_mtime = true;
    let run = finish(engine().start(s, None)).await;
    let path = &run.completed().path;
    assert!(path.starts_with(&nested));
    let secs = std::fs::metadata(path).unwrap().modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    assert_eq!(secs, 784_111_777);
}
}

rt! {
async fn small_file_uses_one_part() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/f/100000"), dir.path());
    s.threads = 8;
    let run = finish(engine().start(s, None)).await;
    assert_eq!(run.max_parts().max(1), 1);
    assert_eq!(sha(&read(&run.completed().path)), expected_hash(1, 100_000));
}
}

rt! {
async fn hls_downloads_segments_in_order() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/hls/master.m3u8"), dir.path());
    s.kind = SourceKind::Hls;
    s.threads = 3;
    let run = finish(engine().start(s, None)).await;
    let done = run.completed();
    assert_eq!(done.file_name, "master.ts");
    let mut want = Vec::new();
    for n in 0..3 {
        want.extend(seg(n));
    }
    assert_eq!(read(&done.path), want);
    assert_eq!(done.size, want.len() as u64);
}
}

rt! {
async fn hls_pause_and_resume() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/hls/high.m3u8"), dir.path());
    s.kind = SourceKind::Hls;
    s.threads = 1;
    let eng = engine();
    let paused_state = ResumeState {
        file_name: "high.ts".into(),
        working_path: dir.path().join("high.ts"),
        resumable: true,
        parts: vec![sdm_engine::Part { from: 0, to: u64::MAX - 1, current: seg(0).len() as u64 }],
        hls_next_segment: Some(1),
        ..ResumeState::default()
    };
    std::fs::write(dir.path().join("high.ts"), seg(0)).unwrap();
    let run = finish(eng.start(s, Some(paused_state))).await;
    let mut want = Vec::new();
    for n in 0..3 {
        want.extend(seg(n));
    }
    assert_eq!(read(&run.completed().path), want);
}
}

rt! {
async fn hls_refuses_encrypted_streams() {
    let (base, _) = serve().await;
    let dir = tempfile::tempdir().unwrap();
    let mut s = spec(format!("{base}/hls/enc.m3u8"), dir.path());
    s.kind = SourceKind::Hls;
    let run = finish(engine().start(s, None)).await;
    match run.last() {
        Event::Failed { error: EngineError::Other(m), .. } => assert!(m.contains("encrypted"), "{m}"),
        other => panic!("expected a plain failure, got {other:?}"),
    }
}
}

#[test]
fn header_value_type_is_available() {
    let _ = HeaderValue::from_static("x");
}
