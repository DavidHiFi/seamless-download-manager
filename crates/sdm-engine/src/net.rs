use std::error::Error as _;

use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_DISPOSITION, CONTENT_RANGE, CONTENT_TYPE, ETAG, HeaderMap, HeaderName,
    HeaderValue, LAST_MODIFIED, RANGE, REFERER, USER_AGENT,
};
use reqwest::{RequestBuilder, Response};

use crate::names::{extension_for_type, sanitize_file_name};
use crate::{
    EngineConfig, EngineError, EngineInner, JobSpec, ProbeInfo, ProxyConfig, Validator,
    file_name_from_disposition, file_name_from_url,
};

pub(crate) fn build_client(cfg: &EngineConfig) -> Result<reqwest::Client, EngineError> {
    let mut b = base_builder(cfg)?;
    if let Some(url) = cfg.doh_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        let parsed = url::Url::parse(url)
            .ok()
            .filter(|u| u.scheme() == "https")
            .ok_or_else(|| EngineError::Other("the DNS address must be an https link".into()))?;
        let doh_client = base_builder(cfg)?
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .map_err(|e| EngineError::Other(format!("could not set up the DNS client: {e}")))?;
        b = b.dns_resolver(crate::doh::DohResolver::new(parsed.to_string(), doh_client));
    }
    b.build().map_err(|e| EngineError::Other(format!("could not set up the network client: {e}")))
}

fn base_builder(cfg: &EngineConfig) -> Result<reqwest::ClientBuilder, EngineError> {
    let mut b = reqwest::Client::builder()
        .use_rustls_tls()
        .connect_timeout(cfg.connect_timeout)
        .read_timeout(cfg.read_timeout)
        .redirect(reqwest::redirect::Policy::limited(10))
        .danger_accept_invalid_certs(cfg.ignore_tls_errors)
        .tcp_nodelay(true);
    match &cfg.proxy {
        ProxyConfig::System => {}
        ProxyConfig::Direct => b = b.no_proxy(),
        ProxyConfig::Manual { url } => {
            let proxy = reqwest::Proxy::all(url.as_str())
                .map_err(|e| EngineError::Other(format!("the proxy address is not valid: {e}")))?;
            b = b.proxy(proxy);
        }
    }
    Ok(b)
}

/// Headers sent on every request of a job (no Range, no auth: those are per request).
pub(crate) fn build_headers(spec: &JobSpec, cfg: &EngineConfig) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in &spec.headers {
        let lower = k.to_ascii_lowercase();
        let dropped = matches!(
            lower.as_str(),
            "host" | "range" | "accept-encoding" | "content-length" | "connection"
        ) || lower.starts_with("if-")
            || lower.starts_with("sec-fetch-");
        if dropped {
            continue;
        }
        let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_bytes(v.as_bytes()),
        ) else {
            continue;
        };
        map.append(name, value);
    }
    if !map.contains_key(REFERER)
        && let Some(r) = &spec.referrer
        && let Ok(v) = HeaderValue::from_str(r)
    {
        map.insert(REFERER, v);
    }
    let ua = spec.user_agent.as_deref().filter(|u| !u.is_empty());
    if let Some(ua) = ua {
        if let Ok(v) = HeaderValue::from_str(ua) {
            map.insert(USER_AGENT, v);
        }
    } else if !map.contains_key(USER_AGENT)
        && let Ok(v) = HeaderValue::from_str(&cfg.user_agent)
    {
        map.insert(USER_AGENT, v);
    }
    map.insert(ACCEPT_ENCODING, HeaderValue::from_static("identity"));
    map
}

pub(crate) fn get(inner: &EngineInner, spec: &JobSpec, headers: &HeaderMap, url: &str) -> RequestBuilder {
    let mut rb = inner.client.get(url).headers(headers.clone());
    if let Some((user, pass)) = &spec.basic_auth {
        rb = rb.basic_auth(user, Some(pass));
    }
    rb
}

pub(crate) fn net_err(e: &reqwest::Error) -> EngineError {
    let mut text = e.to_string();
    let mut src = e.source();
    while let Some(s) = src {
        text.push_str(": ");
        text.push_str(&s.to_string());
        src = s.source();
    }
    EngineError::Network(text)
}

pub(crate) fn retryable(e: &EngineError) -> bool {
    match e {
        EngineError::Network(_) => true,
        EngineError::Http { status } => matches!(status, 408 | 429 | 500..=599),
        _ => false,
    }
}

/// 1, 2, 4, 8, 16, 30, 30 ... seconds for attempt 1, 2, 3 ...
pub(crate) fn backoff(attempt: u32) -> std::time::Duration {
    const STEPS: [u64; 7] = [1, 2, 4, 8, 16, 30, 30];
    let i = (attempt.max(1) as usize - 1).min(STEPS.len() - 1);
    std::time::Duration::from_secs(STEPS[i])
}

pub(crate) fn header_text(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
}

/// `(first byte, total)` from `Content-Range: bytes 0-99/1000`.
pub(crate) fn content_range(headers: &HeaderMap) -> Option<(u64, Option<u64>)> {
    let text = header_text(headers, CONTENT_RANGE)?;
    let rest = text.trim().strip_prefix("bytes")?.trim();
    let (range, total) = rest.split_once('/')?;
    let start = range.split('-').next()?.trim().parse().ok()?;
    Some((start, total.trim().parse().ok()))
}

pub(crate) struct Probed {
    pub info: ProbeInfo,
    pub response: Response,
}

/// One GET with `Range: bytes=0-`. The open response comes back so a job can keep
/// streaming it as part 0.
pub(crate) async fn probe_request(
    inner: &EngineInner,
    spec: &JobSpec,
    headers: &HeaderMap,
) -> Result<Probed, EngineError> {
    url::Url::parse(&spec.url).map_err(|_| EngineError::Other("that is not a valid link".into()))?;
    let mut resp = get(inner, spec, headers, &spec.url)
        .header(RANGE, "bytes=0-")
        .send()
        .await
        .map_err(|e| net_err(&e))?;
    if resp.status().as_u16() == 416 {
        resp = get(inner, spec, headers, &spec.url).send().await.map_err(|e| net_err(&e))?;
    }
    let status = resp.status().as_u16();
    let needs_auth = status == 401;
    if !resp.status().is_success() && !needs_auth {
        return Err(EngineError::Http { status });
    }

    let (total, resumable) = if needs_auth {
        (None, false)
    } else if status == 206 {
        match content_range(resp.headers()) {
            Some((0, Some(total))) => (Some(total), true),
            _ => (None, false),
        }
    } else {
        (resp.content_length(), false)
    };

    let content_type = header_text(resp.headers(), CONTENT_TYPE);
    let is_web_page = content_type
        .as_deref()
        .is_some_and(|c| c.to_ascii_lowercase().contains("text/html"));
    let validator = Validator {
        etag: header_text(resp.headers(), ETAG),
        last_modified: header_text(resp.headers(), LAST_MODIFIED),
    };
    let final_url = resp.url().to_string();
    let disposition = header_text(resp.headers(), CONTENT_DISPOSITION);
    let file_name = pick_file_name(spec, disposition.as_deref(), &final_url, content_type.as_deref());

    Ok(Probed {
        info: ProbeInfo {
            final_url,
            file_name,
            total,
            resumable,
            content_type,
            is_web_page,
            needs_auth,
            validator,
        },
        response: resp,
    })
}

pub(crate) fn pick_file_name(
    spec: &JobSpec,
    disposition: Option<&str>,
    final_url: &str,
    content_type: Option<&str>,
) -> String {
    if let Some(name) = spec.file_name.as_deref().filter(|n| !n.trim().is_empty()) {
        return sanitize_file_name(name);
    }
    let derived = disposition
        .and_then(file_name_from_disposition)
        .or_else(|| file_name_from_url(final_url))
        .unwrap_or_else(|| "download".into());
    let mut name = sanitize_file_name(&derived);
    if std::path::Path::new(&name).extension().is_none()
        && let Some(ext) = content_type.and_then(extension_for_type)
    {
        name.push('.');
        name.push_str(ext);
    }
    name
}
