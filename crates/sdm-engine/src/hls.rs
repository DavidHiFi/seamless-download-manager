use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use url::Url;

use crate::files::{self, disk_err};
use crate::job::{Shell, Stop};
use crate::meter::SpeedMeter;
use crate::names::sanitize_file_name;
use crate::net::{self, backoff, net_err, retryable};
use crate::{
    Completed, Control, EngineError, EngineInner, Event, JobSpec, Part, ProbeInfo, Progress,
    ResumeState, Validator, file_name_from_url,
};

const MAX_PLAYLIST_BYTES: usize = 8 * 1024 * 1024;
const MAX_MASTER_DEPTH: usize = 3;

struct Playlist {
    url: Url,
    segments: Vec<Url>,
}

async fn fetch_text(sh: &mut Shell, url: &Url) -> Result<(String, Url), Stop> {
    let mut failures = 0;
    loop {
        let err = match fetch_text_once(sh, url).await {
            Ok(v) => return Ok(v),
            Err(e) => e,
        };
        if !retryable(&err) || failures >= sh.spec.max_retries {
            return Err(Stop::Failed(err));
        }
        failures += 1;
        sh.emit(Event::Retrying { attempt: failures, error: err });
        if let Some(stop) = sh.wait(backoff(failures)).await {
            return Err(stop);
        }
    }
}

async fn fetch_text_once(sh: &Shell, url: &Url) -> Result<(String, Url), EngineError> {
    let resp = net::get(&sh.inner, &sh.spec, &sh.headers, url.as_str())
        .send()
        .await
        .map_err(|e| net_err(&e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(EngineError::Http { status: status.as_u16() });
    }
    let final_url = resp.url().clone();
    let mut body = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        body.extend_from_slice(&chunk.map_err(|e| net_err(&e))?);
        if body.len() > MAX_PLAYLIST_BYTES {
            return Err(EngineError::Other("that playlist is far too large".into()));
        }
    }
    Ok((String::from_utf8_lossy(&body).into_owned(), final_url))
}

fn attr<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (_, rest) = line.split_once(':')?;
    let mut s = rest;
    while !s.is_empty() {
        let (name, after) = s.split_once('=')?;
        let (value, next) = if let Some(q) = after.strip_prefix('"') {
            let end = q.find('"')?;
            (&q[..end], q[end + 1..].trim_start_matches(','))
        } else {
            match after.split_once(',') {
                Some((v, n)) => (v, n),
                None => (after, ""),
            }
        };
        if name.trim() == key {
            return Some(value);
        }
        s = next;
    }
    None
}

enum Parsed {
    Master(Url),
    Media(Vec<Url>),
}

fn parse(text: &str, base: &Url) -> Result<Parsed, EngineError> {
    let bad = || EngineError::Other("that is not a playable stream playlist".into());
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next() != Some("#EXTM3U") {
        return Err(bad());
    }
    let mut best: Option<(u64, Url)> = None;
    let mut segments = Vec::new();
    let mut pending_variant: Option<u64> = None;
    for line in lines {
        if line.starts_with("#EXT-X-KEY") {
            if attr(line, "METHOD").is_none_or(|m| m != "NONE") {
                return Err(EngineError::Other("this stream is encrypted, and encrypted streams are not supported".into()));
            }
        } else if line.starts_with("#EXT-X-MAP") {
            return Err(EngineError::Other("this stream uses fragmented MP4 pieces, which are not supported".into()));
        } else if line.starts_with("#EXT-X-STREAM-INF") {
            pending_variant = Some(attr(line, "BANDWIDTH").and_then(|b| b.parse().ok()).unwrap_or(0));
        } else if line.starts_with('#') {
        } else {
            let url = base.join(line).map_err(|_| bad())?;
            if let Some(bw) = pending_variant.take() {
                if best.as_ref().is_none_or(|(b, _)| bw > *b) {
                    best = Some((bw, url));
                }
            } else {
                segments.push(url);
            }
        }
    }
    match best {
        Some((_, url)) if segments.is_empty() => Ok(Parsed::Master(url)),
        _ if segments.is_empty() => Err(bad()),
        _ => Ok(Parsed::Media(segments)),
    }
}

async fn load_playlist(sh: &mut Shell) -> Result<Playlist, Stop> {
    let mut url = Url::parse(&sh.spec.url)
        .map_err(|_| Stop::Failed(EngineError::Other("that is not a valid link".into())))?;
    for _ in 0..MAX_MASTER_DEPTH {
        let (text, final_url) = fetch_text(sh, &url).await?;
        match parse(&text, &final_url).map_err(Stop::Failed)? {
            Parsed::Media(segments) => return Ok(Playlist { url: final_url, segments }),
            Parsed::Master(next) => url = next,
        }
    }
    Err(Stop::Failed(EngineError::Other("the playlist points to too many other playlists".into())))
}

struct SegCtx {
    inner: Arc<EngineInner>,
    spec: JobSpec,
    headers: reqwest::header::HeaderMap,
    limit: Arc<crate::SpeedLimiter>,
    tx: mpsc::UnboundedSender<Event>,
}

async fn segment_once(ctx: &SegCtx, url: &Url) -> Result<Vec<u8>, EngineError> {
    let resp = net::get(&ctx.inner, &ctx.spec, &ctx.headers, url.as_str())
        .send()
        .await
        .map_err(|e| net_err(&e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(EngineError::Http { status: status.as_u16() });
    }
    let mut body = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| net_err(&e))?;
        ctx.limit.acquire(chunk.len()).await;
        ctx.inner.cfg.global_limit.acquire(chunk.len()).await;
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn fetch_segment(ctx: Arc<SegCtx>, url: Url) -> Result<Vec<u8>, EngineError> {
    let mut failures = 0;
    loop {
        let err = match segment_once(&ctx, &url).await {
            Ok(b) => return Ok(b),
            Err(e) => e,
        };
        if !retryable(&err) || failures >= ctx.spec.max_retries {
            return Err(err);
        }
        failures += 1;
        let _ = ctx.tx.send(Event::Retrying { attempt: failures, error: err });
        tokio::time::sleep(backoff(failures)).await;
    }
}

fn stream_name(spec: &JobSpec, url: &Url) -> String {
    let base = spec
        .file_name
        .clone()
        .or_else(|| file_name_from_url(url.as_str()))
        .unwrap_or_else(|| "stream".into());
    let mut name = sanitize_file_name(&base);
    let lower = name.to_ascii_lowercase();
    for ext in [".m3u8", ".m3u"] {
        if lower.ends_with(ext) {
            name.truncate(name.len() - ext.len());
        }
    }
    if !name.to_ascii_lowercase().ends_with(".ts") {
        name.push_str(".ts");
    }
    name
}

fn state(file_name: &str, working_path: &std::path::Path, offset: u64, next: u64) -> ResumeState {
    ResumeState {
        file_name: file_name.to_string(),
        working_path: working_path.to_path_buf(),
        total: None,
        resumable: true,
        validator: Validator::default(),
        parts: vec![Part { from: 0, to: u64::MAX - 1, current: offset }],
        hls_next_segment: Some(next),
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
    let mut sh = Shell::new(inner, spec, tx, ctrl);
    let fallback = ResumeState { file_name: sh.spec.file_name.clone().unwrap_or_default(), ..ResumeState::default() };
    match go(&mut sh, resume).await {
        Ok(()) | Err(Stop::Cancelled) => {}
        Err(Stop::Paused) => {
            let _ = events.send(Event::Paused(fallback));
        }
        Err(Stop::Failed(error)) => {
            let _ = events.send(Event::Failed { error, state: fallback });
        }
    }
}

async fn go(sh: &mut Shell, resume: Option<ResumeState>) -> Result<(), Stop> {
    let playlist = load_playlist(sh).await?;
    let total_segments = playlist.segments.len() as u64;

    let resumed = resume.filter(|st| st.hls_next_segment.is_some() && !st.parts.is_empty());
    let (file, working_path, file_name, mut next, mut offset) = match resumed {
        Some(st) => {
            let file = files::open_existing(&st.working_path).map_err(|e| Stop::Failed(disk_err(e)))?;
            let offset = st.parts[0].current;
            let next = st.hls_next_segment.unwrap_or(0).min(total_segments);
            (file, st.working_path, st.file_name, next, offset)
        }
        None => {
            // Named after the link the person added, not the quality variant it pointed to.
            let asked = Url::parse(&sh.spec.url).unwrap_or_else(|_| playlist.url.clone());
            let name = stream_name(&sh.spec, &asked);
            let claimed = files::claim(&sh.spec, &name).map_err(Stop::Failed)?;
            (claimed.file, claimed.working_path, claimed.file_name, 0, 0)
        }
    };
    let file = Arc::new(file);
    sh.emit(Event::Probed(ProbeInfo {
        final_url: playlist.url.to_string(),
        file_name: file_name.clone(),
        total: None,
        resumable: true,
        content_type: Some("application/vnd.apple.mpegurl".into()),
        is_web_page: false,
        needs_auth: false,
        validator: Validator::default(),
    }));

    let ctx = Arc::new(SegCtx {
        inner: sh.inner.clone(),
        spec: sh.spec.clone(),
        headers: sh.headers.clone(),
        limit: sh.limit.clone(),
        tx: sh.tx.clone(),
    });
    let concurrency = sh.threads as usize;
    let segments = playlist.segments;
    let mut fetches = futures_util::stream::iter(next as usize..segments.len())
        .map(|i| {
            let (ctx, url) = (ctx.clone(), segments[i].clone());
            async move { (i as u64, fetch_segment(ctx, url).await) }
        })
        .buffered(concurrency);

    let mut meter = SpeedMeter::new(offset);
    let mut speed = 0;
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    tick.tick().await;
    let progress = |offset: u64, speed: u64, next: u64| Progress {
        downloaded: offset,
        total: None,
        speed,
        active_connections: concurrency.min((total_segments - next) as usize) as u32,
        parts: vec![Part { from: 0, to: u64::MAX - 1, current: offset }],
    };

    loop {
        tokio::select! {
            item = fetches.next() => match item {
                None => break,
                Some((_, Err(error))) => {
                    sh.emit(Event::Failed { error, state: state(&file_name, &working_path, offset, next) });
                    return Ok(());
                }
                Some((index, Ok(bytes))) => {
                    let (f, at, len) = (file.clone(), offset, bytes.len() as u64);
                    let wrote = tokio::task::spawn_blocking(move || files::write_all_at(&f, &bytes, at)).await;
                    if let Err(e) = wrote.map_err(|e| EngineError::Disk(e.to_string())).and_then(|r| r.map_err(disk_err)) {
                        sh.emit(Event::Failed { error: e, state: state(&file_name, &working_path, offset, next) });
                        return Ok(());
                    }
                    offset += len;
                    next = index + 1;
                }
            },
            msg = sh.ctrl.recv() => match msg {
                None => return Err(Stop::Cancelled),
                Some(Control::Pause) => {
                    drop(fetches);
                    let f = file.clone();
                    let _ = tokio::task::spawn_blocking(move || f.sync_data()).await;
                    sh.emit(Event::Paused(state(&file_name, &working_path, offset, next)));
                    return Ok(());
                }
                Some(c) => {
                    sh.apply(c);
                }
            },
            _ = tick.tick() => {
                speed = meter.sample(offset);
                sh.emit(Event::Progress(progress(offset, speed, next)));
            }
        }
    }

    let f = file.clone();
    let synced = tokio::task::spawn_blocking(move || f.sync_data()).await;
    drop(synced);
    drop(file);
    let (path, file_name) = files::finalize(&working_path, &file_name, sh.spec.on_exists).map_err(Stop::Failed)?;
    sh.emit(Event::Progress(progress(offset, speed, next)));
    sh.emit(Event::Completed(Completed { path, file_name, size: offset }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("http://h.test/a/master.m3u8").unwrap()
    }

    #[test]
    fn master_picks_highest_bandwidth() {
        let text = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\nlow.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2400000\nhttp://cdn.test/hi.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=1200000\nmid.m3u8\n";
        match parse(text, &base()).unwrap() {
            Parsed::Master(u) => assert_eq!(u.as_str(), "http://cdn.test/hi.m3u8"),
            Parsed::Media(_) => panic!("expected a master playlist"),
        }
    }

    #[test]
    fn media_lists_segments_in_order() {
        let text = "#EXTM3U\n#EXT-X-KEY:METHOD=NONE\n#EXTINF:4,\ns0.ts\n#EXTINF:4,\n../s1.ts\n#EXT-X-ENDLIST\n";
        match parse(text, &base()).unwrap() {
            Parsed::Media(s) => {
                assert_eq!(s[0].as_str(), "http://h.test/a/s0.ts");
                assert_eq!(s[1].as_str(), "http://h.test/s1.ts");
            }
            Parsed::Master(_) => panic!("expected a media playlist"),
        }
    }

    #[test]
    fn encrypted_and_junk_are_refused() {
        let enc = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k.key\"\n#EXTINF:4,\ns0.ts\n";
        assert!(matches!(parse(enc, &base()), Err(EngineError::Other(m)) if m.contains("encrypted")));
        assert!(parse("<html>", &base()).is_err());
        assert!(parse("#EXTM3U\n", &base()).is_err());
    }

    #[test]
    fn names() {
        let spec = JobSpec::new("http://h.test/live/stream.m3u8", ".");
        let url = Url::parse(&spec.url).unwrap();
        assert_eq!(stream_name(&spec, &url), "stream.ts");
    }
}
