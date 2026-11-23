//! `seamless-download-manager download add ...` and friends: talks to the running copy over the
//! local link the browser extension uses, starting it quietly when it is not running.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const HELP: &str = "\
Seamless Download Manager

  download add --link <url> [--folder <dir>] [--name <file>] [--start]
               [--queue <name or id>] [--category <name or id>] [--header \"Name: value\"]...
  list                 every download with its id
  pause <id>           pause a download
  resume <id>          resume a download
  remove <id> [--delete-file]
  show                 bring the window forward
  gui exit             close the app
";

/// Runs a command line request. `None` when the arguments are not one, so the window starts.
pub fn run(args: &[String], dir: &Path) -> Option<i32> {
    let first = args.first()?.as_str();
    if !matches!(first, "download" | "list" | "pause" | "resume" | "remove" | "show" | "gui" | "help" | "--help" | "-h" | "/?") {
        return None;
    }
    attach_console();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().ok()?;
    let code = match rt.block_on(dispatch(args, dir)) {
        Ok(text) => {
            if !text.is_empty() {
                println!("{text}");
            }
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    Some(code)
}

/// A window app has no console. Borrow the one we were started from, when there is one and
/// output is not already going to a file or pipe.
fn attach_console() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE};
    unsafe {
        let redirected = GetStdHandle(STD_OUTPUT_HANDLE).map(|h| !h.is_invalid() && !h.0.is_null()).unwrap_or(false);
        if !redirected {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

struct Link {
    base: String,
    key: String,
    client: reqwest::Client,
}

impl Link {
    fn new(dir: &Path) -> Result<Link, String> {
        let settings = sdm_core::Settings::load(&dir.join("settings.json")).map_err(|e| format!("Could not read the settings: {e}"))?;
        if !settings.api_enabled {
            return Err("The browser link is off in Settings, so the command line cannot reach the app.".into());
        }
        Ok(Link {
            base: format!("http://127.0.0.1:{}", settings.api_port),
            key: settings.api_key,
            client: reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| e.to_string())?,
        })
    }

    async fn up(&self) -> bool {
        self.client.post(format!("{}/ping", self.base)).timeout(Duration::from_secs(2)).send().await.is_ok()
    }

    /// Starts the app in the tray when it is not running, and waits for it to answer.
    async fn ensure_running(&self) -> Result<(), String> {
        if self.up().await {
            return Ok(());
        }
        use std::os::windows::process::CommandExt;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::process::Command::new(exe)
            .arg("--background")
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("Could not start the app: {e}"))?;
        let until = Instant::now() + Duration::from_secs(15);
        while Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if self.up().await {
                return Ok(());
            }
        }
        Err("The app did not start in time.".into())
    }

    async fn call(&self, method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Value, String> {
        let mut req = self.client.request(method, format!("{}{}", self.base, path)).header("X-Api-Key", &self.key);
        if let Some(body) = body {
            req = req.json(&body);
        }
        let resp = req.send().await.map_err(|e| format!("Could not reach the app: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if !status.is_success() {
            let why = value.get("error").and_then(Value::as_str).map(str::to_string).unwrap_or(text);
            return Err(format!("The app said no: {why}"));
        }
        Ok(value)
    }
}

/// Pulls `--name value` and bare flags out of the arguments.
struct Opts {
    pairs: Vec<(String, String)>,
    flags: Vec<String>,
    rest: Vec<String>,
}

impl Opts {
    fn parse(args: &[String], with_value: &[&str]) -> Result<Opts, String> {
        let mut opts = Opts { pairs: Vec::new(), flags: Vec::new(), rest: Vec::new() };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if let Some(name) = a.strip_prefix("--") {
                let (name, inline) = match name.split_once('=') {
                    Some((n, v)) => (n, Some(v.to_string())),
                    None => (name, None),
                };
                if with_value.contains(&name) {
                    let value = match inline {
                        Some(v) => v,
                        None => it.next().cloned().ok_or_else(|| format!("--{name} needs a value."))?,
                    };
                    opts.pairs.push((name.to_string(), value));
                } else {
                    opts.flags.push(name.to_string());
                }
            } else {
                opts.rest.push(a.clone());
            }
        }
        Ok(opts)
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.pairs.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    fn all(&self, name: &str) -> impl Iterator<Item = &str> {
        self.pairs.iter().filter(move |(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }
}

fn id_arg(rest: &[String], what: &str) -> Result<i64, String> {
    rest.first().and_then(|s| s.parse::<i64>().ok()).ok_or_else(|| format!("Give the download's number, like: {what} 12"))
}

/// A queue or file type picked by number or by name, case does not matter.
async fn pick(link: &Link, path: &str, wanted: &str, what: &str) -> Result<i64, String> {
    if let Ok(id) = wanted.trim().parse::<i64>() {
        return Ok(id);
    }
    let list = link.call(reqwest::Method::GET, path, None).await?;
    list.as_array()
        .into_iter()
        .flatten()
        .find(|v| v.get("name").and_then(Value::as_str).is_some_and(|n| n.eq_ignore_ascii_case(wanted.trim())))
        .and_then(|v| v.get("id").and_then(Value::as_i64))
        .ok_or_else(|| format!("There is no {what} called {wanted}."))
}

async fn dispatch(args: &[String], dir: &Path) -> Result<String, String> {
    let cmd = args[0].as_str();
    let rest = &args[1..];
    match cmd {
        "help" | "--help" | "-h" | "/?" => Ok(HELP.trim_end().to_string()),
        "download" => {
            if rest.first().map(String::as_str) != Some("add") {
                return Err(format!("Try: download add --link <url>\n\n{HELP}"));
            }
            let o = Opts::parse(&rest[1..], &["link", "folder", "name", "queue", "category", "header"])?;
            let url = o.get("link").or(o.rest.first().map(String::as_str)).ok_or("download add needs --link <url>.")?;
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err("The link must start with http:// or https://.".into());
            }
            let mut headers = serde_json::Map::new();
            for h in o.all("header") {
                let (k, v) = h.split_once(':').ok_or_else(|| format!("A header looks like \"Name: value\", not {h}."))?;
                headers.insert(k.trim().to_string(), Value::String(v.trim().to_string()));
            }
            let link = Link::new(dir)?;
            link.ensure_running().await?;
            let queue = match o.get("queue") {
                Some(q) => Some(pick(&link, "/queues", q, "queue").await?),
                None => None,
            };
            let category = match o.get("category") {
                Some(c) => Some(pick(&link, "/v1/categories", c, "file type").await?),
                None => None,
            };
            let start = o.flag("start");
            let body = json!({
                "downloadSource": { "link": url, "headers": headers },
                "folder": o.get("folder"),
                "name": o.get("name"),
                "queueId": queue,
                "categoryId": category,
                "startDownload": start && queue.is_none(),
                "startQueue": start && queue.is_some(),
            });
            let answer = link.call(reqwest::Method::POST, "/start-headless-download", Some(body)).await?;
            let id = answer.get("id").and_then(Value::as_i64).unwrap_or_default();
            Ok(format!("Added download {id}{}", if start { " and started it." } else { "." }))
        }
        "list" => {
            let link = Link::new(dir)?;
            if !link.up().await {
                return Err("The app is not running.".into());
            }
            let list = link.call(reqwest::Method::GET, "/v1/downloads", None).await?;
            let mut out = String::new();
            for v in list.as_array().into_iter().flatten() {
                let id = v.get("id").and_then(Value::as_i64).unwrap_or_default();
                let status = v.get("status").and_then(Value::as_str).unwrap_or("");
                let name = v.get("name").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!("{id:>6}  {status:<12} {name}\n"));
            }
            Ok(out.trim_end().to_string())
        }
        "pause" | "resume" | "remove" => {
            let id = id_arg(rest, cmd)?;
            let link = Link::new(dir)?;
            if !link.up().await {
                return Err("The app is not running.".into());
            }
            match cmd {
                "remove" => {
                    let file = if rest.iter().any(|a| a == "--delete-file") { "?file=1" } else { "" };
                    link.call(reqwest::Method::DELETE, &format!("/v1/downloads/{id}{file}"), None).await?;
                    Ok(format!("Removed download {id}."))
                }
                _ => {
                    link.call(reqwest::Method::POST, &format!("/v1/downloads/{id}/{cmd}"), None).await?;
                    Ok(format!("{} download {id}.", if cmd == "pause" { "Paused" } else { "Resumed" }))
                }
            }
        }
        "show" => {
            let link = Link::new(dir)?;
            link.ensure_running().await?;
            link.call(reqwest::Method::POST, "/v1/show", None).await?;
            Ok(String::new())
        }
        "gui" => {
            if rest.first().map(String::as_str) != Some("exit") {
                return Err("Try: gui exit".into());
            }
            let link = Link::new(dir)?;
            if !link.up().await {
                return Ok("The app is not running.".into());
            }
            link.call(reqwest::Method::POST, "/v1/exit", None).await?;
            Ok("Closing the app.".into())
        }
        _ => Err(HELP.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn options_parse() {
        let o = Opts::parse(&s(&["--link", "https://a/b", "--start", "--header=A: 1", "--header", "B: 2", "x"]), &["link", "header"]).unwrap();
        assert_eq!(o.get("link"), Some("https://a/b"));
        assert!(o.flag("start"));
        assert_eq!(o.all("header").collect::<Vec<_>>(), ["A: 1", "B: 2"]);
        assert_eq!(o.rest, ["x"]);
        assert!(Opts::parse(&s(&["--link"]), &["link"]).is_err());
    }

    #[test]
    fn plain_launch_is_not_a_command() {
        let dir = std::env::temp_dir();
        assert!(run(&[], &dir).is_none());
        assert!(run(&s(&["--background"]), &dir).is_none());
        assert!(run(&s(&["https://example.com/a.zip"]), &dir).is_none());
    }
}
