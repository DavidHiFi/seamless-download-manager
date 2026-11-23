use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sdm_core::manager::{AddRequest, Manager, ManagerEvent};
use sdm_engine::SourceKind;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const API_KEY_HEADER: &str = "x-api-key";

#[derive(Clone)]
struct AppState {
    manager: Arc<Manager>,
}

struct ApiError(StatusCode, String);

impl ApiError {
    fn bad(message: impl Into<String>) -> ApiError {
        ApiError(StatusCode::BAD_REQUEST, message.into())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> ApiError {
        ApiError(StatusCode::BAD_REQUEST, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"ok": false, "error": self.1}))).into_response()
    }
}

type ApiResult = Result<Json<Value>, ApiError>;

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() ^ b.len()) as u32;
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= (x ^ y) as u32;
    }
    diff == 0
}

async fn require_key(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let expected = state.manager.settings().api_key;
    let given = req.headers().get(API_KEY_HEADER).and_then(|v| v.to_str().ok()).unwrap_or("");
    if expected.is_empty() || !constant_time_eq(given.as_bytes(), expected.as_bytes()) {
        return ApiError(StatusCode::UNAUTHORIZED, "missing or wrong api key".into()).into_response();
    }
    next.run(req).await
}

fn parse_body(body: &Bytes) -> Result<Value, ApiError> {
    serde_json::from_slice(body).map_err(|_| ApiError::bad("the request body is not valid JSON"))
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn parse_headers(v: Option<&Value>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    match v {
        Some(Value::Object(map)) => {
            for (k, val) in map {
                if let Some(s) = val.as_str() {
                    out.push((k.clone(), s.to_string()));
                }
            }
        }
        Some(Value::Array(list)) => {
            for entry in list {
                match entry {
                    Value::Array(pair) if pair.len() == 2 => {
                        if let (Some(k), Some(val)) = (pair[0].as_str(), pair[1].as_str()) {
                            out.push((k.to_string(), val.to_string()));
                        }
                    }
                    Value::Object(_) => {
                        let k = entry.get("name").or_else(|| entry.get("key")).and_then(Value::as_str);
                        let val = entry.get("value").and_then(Value::as_str);
                        if let (Some(k), Some(val)) = (k, val) {
                            out.push((k.to_string(), val.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    out.retain(|(k, _)| !k.trim().is_empty());
    out
}

fn kind_of(v: &Value) -> SourceKind {
    match v.get("type").and_then(Value::as_str).map(str::to_ascii_lowercase).as_deref() {
        Some("hls") => SourceKind::Hls,
        _ => SourceKind::Http,
    }
}

fn request_from_source(source: &Value) -> Result<AddRequest, ApiError> {
    let url = str_field(source, "link").or_else(|| str_field(source, "url")).ok_or_else(|| ApiError::bad("an item needs a link"))?;
    Ok(AddRequest {
        url,
        kind: kind_of(source),
        headers: parse_headers(source.get("headers")),
        referrer: str_field(source, "downloadPage"),
        suggested_name: str_field(source, "suggestedName"),
        ..Default::default()
    })
}

async fn ping() -> Json<Value> {
    Json(json!({"ok": true, "app": "sdm", "version": env!("CARGO_PKG_VERSION")}))
}

async fn add(State(state): State<AppState>, body: Bytes) -> ApiResult {
    let value = parse_body(&body)?;
    let (items, options) = match &value {
        Value::Array(list) => (list.clone(), Value::Null),
        Value::Object(_) => match value.get("items") {
            Some(Value::Array(list)) => (list.clone(), value.get("options").cloned().unwrap_or(Value::Null)),
            _ => return Err(ApiError::bad("expected an items list")),
        },
        _ => return Err(ApiError::bad("expected an items list")),
    };
    if items.is_empty() {
        return Err(ApiError::bad("the items list is empty"));
    }
    let flag = |name: &str| options.get(name).and_then(Value::as_bool);
    let silent_add = flag("silentAdd").unwrap_or(false);
    let silent_start = flag("silentStart").unwrap_or(false);

    let mut requests = Vec::new();
    for item in &items {
        requests.push(request_from_source(item)?);
    }

    let mut ids = Vec::new();
    for mut req in requests {
        if silent_add {
            req.start = silent_start;
            ids.push(state.manager.add(req)?);
        } else {
            state.manager.request_add(req);
        }
    }
    Ok(Json(json!({"ok": true, "ids": ids})))
}

async fn queues(State(state): State<AppState>) -> Json<Value> {
    let list: Vec<Value> = state.manager.queues().iter().map(|q| json!({"id": q.id, "name": q.name})).collect();
    Json(Value::Array(list))
}

async fn start_headless(State(state): State<AppState>, body: Bytes) -> ApiResult {
    let value = parse_body(&body)?;
    let source = value.get("downloadSource").ok_or_else(|| ApiError::bad("downloadSource is missing"))?;
    let mut req = request_from_source(source)?;
    req.folder = str_field(&value, "folder");
    req.name = str_field(&value, "name");
    req.queue_id = value.get("queueId").and_then(Value::as_i64);
    req.category_id = value.get("categoryId").and_then(Value::as_i64);
    req.start = value.get("startDownload").and_then(Value::as_bool).unwrap_or(false);
    req.start_queue = value.get("startQueue").and_then(Value::as_bool).unwrap_or(false);
    if let Some(user) = str_field(&value, "username") {
        req.basic_auth = Some((user, value.get("password").and_then(Value::as_str).unwrap_or("").to_string()));
    }
    let id = state.manager.add(req)?;
    Ok(Json(json!({"ok": true, "id": id})))
}

fn view_json(v: &sdm_core::manager::DownloadView) -> Value {
    let progress = match v.size {
        Some(size) if size > 0 => (v.downloaded as f64 / size as f64).clamp(0.0, 1.0),
        _ => 0.0,
    };
    json!({
        "id": v.id,
        "name": v.name,
        "folder": v.folder,
        "url": v.url,
        "size": v.size,
        "downloaded": v.downloaded,
        "progress": progress,
        "status": v.status.as_str(),
        "error": v.error,
        "speed": v.speed,
        "etaSecs": v.eta_secs,
        "dateAdded": v.date_added,
        "dateFinished": v.date_finished,
        "queueId": v.queue_id,
        "categoryId": v.category_id,
        "connections": v.connections,
    })
}

async fn list_downloads(State(state): State<AppState>) -> Json<Value> {
    Json(Value::Array(state.manager.list().iter().map(view_json).collect()))
}

fn existing(state: &AppState, id: i64) -> Result<(), ApiError> {
    state.manager.get(id).map(|_| ()).ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "no such download".into()))
}

async fn pause(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult {
    existing(&state, id)?;
    state.manager.pause(id);
    Ok(Json(json!({"ok": true})))
}

async fn resume(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult {
    existing(&state, id)?;
    state.manager.resume(id);
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct SpeedLimitBody {
    /// Bytes per second; 0 lifts the limit.
    rate: u64,
}

async fn speed_limit(State(state): State<AppState>, Path(id): Path<i64>, Json(body): Json<SpeedLimitBody>) -> ApiResult {
    existing(&state, id)?;
    state.manager.set_speed_limit(id, body.rate);
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
struct RemoveQuery {
    file: Option<String>,
}

async fn remove(State(state): State<AppState>, Path(id): Path<i64>, Query(q): Query<RemoveQuery>) -> ApiResult {
    existing(&state, id)?;
    let delete_file = matches!(q.file.as_deref(), Some("1" | "true" | "yes"));
    state.manager.remove(id, delete_file)?;
    Ok(Json(json!({"ok": true})))
}

async fn handoff(State(state): State<AppState>, body: Bytes) -> ApiResult {
    let value = parse_body(&body)?;
    let path = str_field(&value, "path").ok_or_else(|| ApiError::bad("path is missing"))?;
    let id = state.manager.adopt_file(&PathBuf::from(path), str_field(&value, "url"), str_field(&value, "referrer"))?;
    Ok(Json(json!({"ok": true, "id": id})))
}

async fn categories(State(state): State<AppState>) -> Json<Value> {
    let list: Vec<Value> = state.manager.categories().iter().map(|c| json!({"id": c.id, "name": c.name})).collect();
    Json(Value::Array(list))
}

async fn exit(State(state): State<AppState>) -> Json<Value> {
    state.manager.request_exit();
    Json(json!({"ok": true}))
}

async fn show(State(state): State<AppState>) -> Json<Value> {
    state.manager.request_show();
    Json(json!({"ok": true}))
}

async fn not_found() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "no such route".into())
}

fn router(manager: Arc<Manager>) -> Router {
    let state = AppState { manager };
    let protected = Router::new()
        .route("/add", post(add))
        .route("/queues", get(queues))
        .route("/start-headless-download", post(start_headless))
        .route("/v1/downloads", get(list_downloads))
        .route("/v1/downloads/{id}/pause", post(pause))
        .route("/v1/downloads/{id}/resume", post(resume))
        .route("/v1/downloads/{id}/speed-limit", post(speed_limit))
        .route("/v1/downloads/{id}", axum::routing::delete(remove))
        .route("/v1/handoff", post(handoff))
        .route("/v1/show", post(show))
        .route("/v1/categories", get(categories))
        .route("/v1/exit", post(exit))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_key));
    Router::new().route("/ping", post(ping)).merge(protected).fallback(not_found).with_state(state)
}

struct Server {
    port: u16,
    stop: oneshot::Sender<()>,
    join: JoinHandle<()>,
}

async fn bind(manager: &Arc<Manager>, port: u16) -> Result<Server> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await.with_context(|| format!("could not listen on port {port}"))?;
    let port = listener.local_addr()?.port();
    let (stop, stopped) = oneshot::channel::<()>();
    let app = router(manager.clone());
    let join = tokio::spawn(async move {
        let result = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = stopped.await;
            })
            .await;
        if let Err(e) = result {
            tracing::warn!("api server stopped: {e}");
        }
    });
    Ok(Server { port, stop, join })
}

pub struct ApiHandle {
    port: Arc<std::sync::atomic::AtomicU16>,
    stop: Option<oneshot::Sender<()>>,
    supervisor: JoinHandle<()>,
}

impl ApiHandle {
    pub fn port(&self) -> u16 {
        self.port.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Stops listening. Requests already running finish first.
    pub async fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let _ = self.supervisor.await;
    }
}

/// Listens on `127.0.0.1:<api_port>` from the manager's settings. A changed port is picked up
/// live, and the key is read on every request. Port 0 picks any free port.
pub async fn serve(manager: Arc<Manager>) -> Result<ApiHandle> {
    let first = bind(&manager, manager.settings().api_port).await?;
    let port = Arc::new(std::sync::atomic::AtomicU16::new(first.port));
    let mut events = manager.subscribe();
    let (stop, mut stopped) = oneshot::channel::<()>();
    let shared = port.clone();
    let supervisor_manager = manager.clone();
    let supervisor = tokio::spawn(async move {
        let mut current = first;
        loop {
            tokio::select! {
                _ = &mut stopped => break,
                event = events.recv() => {
                    match event {
                        Ok(ManagerEvent::SettingsChanged) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let wanted = supervisor_manager.settings().api_port;
                            if wanted != 0 && wanted != current.port {
                                match bind(&supervisor_manager, wanted).await {
                                    Ok(next) => {
                                        let old = std::mem::replace(&mut current, next);
                                        shared.store(current.port, std::sync::atomic::Ordering::SeqCst);
                                        let _ = old.stop.send(());
                                        let _ = old.join.await;
                                    }
                                    Err(e) => tracing::warn!("could not move the api to port {wanted}: {e}"),
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            let _ = (&mut stopped).await;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        let _ = current.stop.send(());
        let _ = current.join.await;
    });
    Ok(ApiHandle { port, stop: Some(stop), supervisor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    async fn start() -> (tempfile::TempDir, Arc<Manager>, ApiHandle, String) {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::open(dir.path().to_path_buf()).unwrap();
        let mut settings = manager.settings();
        settings.api_port = 0;
        manager.update_settings(settings).unwrap();
        let handle = serve(manager.clone()).await.unwrap();
        let key = manager.settings().api_key;
        (dir, manager, handle, key)
    }

    async fn call(port: u16, method: &str, path: &str, key: Option<&str>, body: Option<&str>) -> (u16, Value) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let body = body.unwrap_or("");
        let key_line = key.map(|k| format!("X-Api-Key: {k}\r\n")).unwrap_or_default();
        let req = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{key_line}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        let text = String::from_utf8_lossy(&raw).into_owned();
        let status: u16 = text.split_whitespace().nth(1).unwrap().parse().unwrap();
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    }

    #[tokio::test]
    async fn ping_needs_no_key() {
        let (_d, _m, handle, _key) = start().await;
        let (status, body) = call(handle.port(), "POST", "/ping", None, Some("{}")).await;
        assert_eq!(status, 200);
        assert_eq!(body["ok"], true);
        assert_eq!(body["app"], "sdm");
        handle.stop().await;
    }

    #[tokio::test]
    async fn other_routes_refuse_without_the_key() {
        let (_d, _m, handle, key) = start().await;
        let port = handle.port();
        let (status, body) = call(port, "GET", "/queues", None, None).await;
        assert_eq!(status, 401);
        assert_eq!(body["ok"], false);
        let (status, _) = call(port, "GET", "/queues", Some("wrong"), None).await;
        assert_eq!(status, 401);
        let (status, _) = call(port, "GET", "/v1/downloads", Some(&key), None).await;
        assert_eq!(status, 200);
        handle.stop().await;
    }

    #[tokio::test]
    async fn silent_add_creates_an_item() {
        let (_d, manager, handle, key) = start().await;
        let port = handle.port();
        let body = r#"{"items":[{"type":"http","link":"https://example.com/a.zip","headers":{"Cookie":"x=1"},"downloadPage":"https://example.com/","suggestedName":"a.zip"}],"options":{"silentAdd":true,"silentStart":false}}"#;
        let (status, reply) = call(port, "POST", "/add", Some(&key), Some(body)).await;
        assert_eq!(status, 200);
        let id = reply["ids"][0].as_i64().unwrap();
        let view = manager.get(id).unwrap();
        assert_eq!(view.name, "a.zip");
        let (_, list) = call(port, "GET", "/v1/downloads", Some(&key), None).await;
        assert_eq!(list[0]["status"], "added");
        handle.stop().await;
    }

    #[tokio::test]
    async fn loud_add_asks_the_window() {
        let (_d, manager, handle, key) = start().await;
        let mut rx = manager.subscribe();
        let body = r#"[{"type":"http","link":"https://example.com/b.zip"}]"#;
        let (status, reply) = call(handle.port(), "POST", "/add", Some(&key), Some(body)).await;
        assert_eq!(status, 200);
        assert_eq!(reply["ids"], json!([]));
        assert!(manager.list().is_empty());
        assert!(matches!(rx.try_recv(), Ok(ManagerEvent::AddRequested(r)) if r.url == "https://example.com/b.zip"));
        handle.stop().await;
    }

    #[tokio::test]
    async fn queues_list_and_bad_input() {
        let (_d, _m, handle, key) = start().await;
        let port = handle.port();
        let (status, body) = call(port, "GET", "/queues", Some(&key), None).await;
        assert_eq!(status, 200);
        assert_eq!(body, json!([{"id": 0, "name": "Main"}]));
        let (status, body) = call(port, "POST", "/add", Some(&key), Some("not json")).await;
        assert_eq!(status, 400);
        assert_eq!(body["ok"], false);
        let (status, _) = call(port, "POST", "/v1/downloads/99/pause", Some(&key), None).await;
        assert_eq!(status, 404);
        handle.stop().await;
    }

    #[tokio::test]
    async fn handoff_adopts_a_file() {
        let (dir, manager, handle, key) = start().await;
        let file = dir.path().join("done.bin");
        std::fs::write(&file, b"abc").unwrap();
        let body = json!({"path": file.to_string_lossy(), "url": "https://example.com/done.bin"}).to_string();
        let (status, reply) = call(handle.port(), "POST", "/v1/handoff", Some(&key), Some(&body)).await;
        assert_eq!(status, 200);
        let id = reply["id"].as_i64().unwrap();
        assert_eq!(manager.get(id).unwrap().downloaded, 3);
        let (status, _) = call(handle.port(), "DELETE", &format!("/v1/downloads/{id}"), Some(&key), None).await;
        assert_eq!(status, 200);
        assert!(manager.get(id).is_none());
        assert!(file.exists());
        handle.stop().await;
    }

    #[tokio::test]
    async fn exit_and_headless_sign_in() {
        let (_d, manager, handle, key) = start().await;
        let port = handle.port();
        let mut settings = manager.settings();
        settings.per_host = vec![sdm_core::PerHostRule {
            host: "*.example.com".into(),
            user_agent: "Site agent".into(),
            threads: 2,
            speed_limit: 1000,
            ..Default::default()
        }];
        manager.update_settings(settings).unwrap();
        let mut rx = manager.subscribe();
        let body = json!({
            "downloadSource": {"link": "https://files.example.com/c.zip"},
            "username": "me", "password": "pw", "startDownload": false
        })
        .to_string();
        let (status, reply) = call(port, "POST", "/start-headless-download", Some(&key), Some(&body)).await;
        assert_eq!(status, 200);
        let id = reply["id"].as_i64().unwrap();
        let item = manager.item(id).unwrap();
        assert_eq!(item.basic_auth, Some(("me".into(), "pw".into())));
        assert_eq!(item.user_agent.as_deref(), Some("Site agent"));
        assert_eq!(item.threads, Some(2));
        assert_eq!(item.speed_limit, 1000);
        let (status, _) = call(port, "POST", "/v1/exit", Some(&key), Some("{}")).await;
        assert_eq!(status, 200);
        let mut saw_exit = false;
        while let Ok(event) = rx.try_recv() {
            saw_exit |= matches!(event, ManagerEvent::ExitRequested);
        }
        assert!(saw_exit);
        let (status, cats) = call(port, "GET", "/v1/categories", Some(&key), None).await;
        assert_eq!(status, 200);
        assert!(cats.as_array().is_some_and(|c| !c.is_empty()));
        handle.stop().await;
    }

    #[test]
    fn constant_time_compare() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
