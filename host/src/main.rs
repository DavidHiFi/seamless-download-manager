mod framing;
mod http;
mod ids;
mod install;

use std::io::{self, Write};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DETACHED_PROCESS: u32 = 0x0000_0008;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Config {
    port: u16,
    api_key: String,
}

trait Backend {
    fn config(&self) -> Result<Config>;
    /// Makes sure the app answers, starting it when it does not.
    fn ensure_running(&self) -> Result<()>;
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value>;
}

struct RealBackend;

fn load_config() -> Result<Config> {
    let path = install::data_dir()?.join("settings.json");
    let text = std::fs::read_to_string(&path).map_err(|_| anyhow!("the app has not been started yet"))?;
    parse_config(&text)
}

fn parse_config(text: &str) -> Result<Config> {
    let value: Value = serde_json::from_str(text).map_err(|_| anyhow!("the app settings could not be read"))?;
    let port = value.get("api_port").and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok()).filter(|p| *p != 0).unwrap_or(15161);
    let api_key = value.get("api_key").and_then(Value::as_str).unwrap_or("").to_string();
    if api_key.is_empty() {
        bail!("the app has no api key yet");
    }
    Ok(Config { port, api_key })
}

fn ping(port: u16) -> bool {
    matches!(http::request(port, "POST", "/ping", None, Some(&json!({}))), Ok((200, v)) if v["ok"] == true)
}

fn start_app() -> Result<()> {
    let exe = install::data_dir()?.join("bin").join("seamless-download-manager.exe");
    if !exe.exists() {
        bail!("the app is not installed");
    }
    Command::new(exe)
        .arg("--background")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()
        .map_err(|_| anyhow!("the app could not be started"))?;
    Ok(())
}

impl Backend for RealBackend {
    fn config(&self) -> Result<Config> {
        load_config()
    }

    fn ensure_running(&self) -> Result<()> {
        let started = Instant::now();
        let mut port = load_config().map(|c| c.port).unwrap_or(15161);
        if ping(port) {
            return Ok(());
        }
        start_app()?;
        while started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(250));
            if let Ok(config) = load_config() {
                port = config.port;
            }
            if ping(port) {
                return Ok(());
            }
        }
        bail!("the app did not start in time")
    }

    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        let config = load_config()?;
        let (status, reply) = http::request(config.port, method, path, Some(&config.api_key), body)?;
        if status >= 400 || reply["ok"] == false {
            let reason = reply["error"].as_str().map(str::to_string).unwrap_or_else(|| format!("the app answered with {status}"));
            bail!(reason);
        }
        Ok(reply)
    }
}

fn dispatch(backend: &dyn Backend, action: &str, payload: Value) -> Result<Value> {
    match action {
        "config" => {
            let config = backend.config()?;
            Ok(json!({"port": config.port, "apiKey": config.api_key}))
        }
        "ping" => {
            backend.ensure_running()?;
            let config = backend.config()?;
            let (_, reply) = http::request(config.port, "POST", "/ping", None, Some(&json!({})))?;
            Ok(reply)
        }
        "add" => {
            backend.ensure_running()?;
            backend.call("POST", "/add", Some(&payload))
        }
        "handoff" => {
            backend.ensure_running()?;
            backend.call("POST", "/v1/handoff", Some(&payload))
        }
        other => bail!("unknown action {other:?}"),
    }
}

fn handle(backend: &dyn Backend, raw: &[u8]) -> Value {
    let request: Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return json!({"id": Value::Null, "ok": false, "error": "the request is not valid JSON"}),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(action) = request.get("action").and_then(Value::as_str) else {
        return json!({"id": id, "ok": false, "error": "the request has no action"});
    };
    let payload = request.get("payload").cloned().unwrap_or(Value::Null);
    match dispatch(backend, action, payload) {
        Ok(result) => json!({"id": id, "ok": true, "result": result}),
        Err(e) => json!({"id": id, "ok": false, "error": e.to_string()}),
    }
}

fn run_host() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    while let Some(raw) = framing::read_message(&mut input)? {
        let reply = handle(&RealBackend, &raw);
        let bytes = serde_json::to_vec(&reply)?;
        match framing::write_message(&mut output, &bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                let fallback = json!({"id": reply["id"], "ok": false, "error": "the reply was too large"});
                framing::write_message(&mut output, &serde_json::to_vec(&fallback)?)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    output.flush()?;
    Ok(())
}

fn main() {
    let arg = std::env::args().nth(1);
    let result = match arg.as_deref() {
        Some("--install") => install::install(),
        Some("--uninstall") => install::uninstall(),
        _ => run_host(),
    };
    if let Err(e) = result {
        eprintln!("sdm-host: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        calls: RefCell<Vec<(String, String)>>,
        fail_start: bool,
    }

    impl Backend for Fake {
        fn config(&self) -> Result<Config> {
            Ok(Config { port: 15161, api_key: "FAKE".into() })
        }
        fn ensure_running(&self) -> Result<()> {
            if self.fail_start { bail!("the app did not start in time") } else { Ok(()) }
        }
        fn call(&self, method: &str, path: &str, _body: Option<&Value>) -> Result<Value> {
            self.calls.borrow_mut().push((method.into(), path.into()));
            Ok(json!({"ok": true, "ids": [1]}))
        }
    }

    fn fake(fail_start: bool) -> Fake {
        Fake { calls: RefCell::new(Vec::new()), fail_start }
    }

    #[test]
    fn config_action_returns_port_and_key() {
        let reply = handle(&fake(false), br#"{"id":7,"action":"config"}"#);
        assert_eq!(reply, json!({"id": 7, "ok": true, "result": {"port": 15161, "apiKey": "FAKE"}}));
    }

    #[test]
    fn add_and_handoff_route_to_the_api() {
        let backend = fake(false);
        let reply = handle(&backend, br#"{"id":"a","action":"add","payload":{"items":[]}}"#);
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["id"], "a");
        assert_eq!(reply["result"]["ids"], json!([1]));
        handle(&backend, br#"{"id":2,"action":"handoff","payload":{"path":"x"}}"#);
        assert_eq!(*backend.calls.borrow(), vec![("POST".to_string(), "/add".to_string()), ("POST".to_string(), "/v1/handoff".to_string())]);
    }

    #[test]
    fn failures_come_back_as_errors() {
        let reply = handle(&fake(true), br#"{"id":3,"action":"add","payload":{}}"#);
        assert_eq!(reply, json!({"id": 3, "ok": false, "error": "the app did not start in time"}));
        assert_eq!(handle(&fake(false), br#"{"id":4,"action":"nope"}"#)["ok"], false);
        assert_eq!(handle(&fake(false), br#"{"id":5}"#)["ok"], false);
        assert_eq!(handle(&fake(false), b"not json")["ok"], false);
    }

    #[test]
    fn settings_parse() {
        let config = parse_config(r#"{"api_port":15200,"api_key":"K","thread_count":8}"#).unwrap();
        assert_eq!(config, Config { port: 15200, api_key: "K".into() });
        assert_eq!(parse_config(r#"{"api_key":"K"}"#).unwrap().port, 15161);
        assert!(parse_config(r#"{"api_key":""}"#).is_err());
        assert!(parse_config("{").is_err());
    }
}
