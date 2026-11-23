#![windows_subsystem = "windows"]
//! Installer and remover for Seamless Download Manager. Installs for the current user only:
//! the programs go in `%LOCALAPPDATA%\SeamlessDownloadManager\bin`, next to the app's data.
//!
//! `--silent` installs (or with `--uninstall`, removes) without a window, for scripted tests.
//! Silent options: `--no-autostart`, `--no-start-menu`, `--no-browsers`, `--open`,
//! `--delete-data`. Exit code 0 when it worked, 1 when not; `setup-log.txt` in `%TEMP%` says why.

mod sys;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use slint::ComponentHandle;

slint::include_modules!();

const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_EXE: &str = "seamless-download-manager.exe";
const HOST_EXE: &str = "sdm-host.exe";
const UNINSTALL_EXE: &str = "uninstall.exe";
const APP_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/seamless-download-manager.exe"));
const HOST_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sdm-host.exe"));

#[derive(Clone, Copy)]
struct Choices {
    autostart: bool,
    start_menu: bool,
    browsers: bool,
    open_after: bool,
    delete_data: bool,
}

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("SeamlessDownloadManager")
}

fn bin_dir() -> PathBuf {
    data_dir().join("bin")
}

fn log(line: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("seamless-download-manager-setup-log.txt");
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let removing = has("--uninstall");

    // The remover lives in the folder it deletes, so it runs from a copy in the temp folder.
    if removing && !has("--from-temp") {
        match sys::relaunch_from_temp(&args) {
            Ok(()) => return,
            Err(e) => log(&format!("could not copy the remover to the temp folder: {e:#}")),
        }
    }

    if has("--silent") {
        let choices = Choices {
            autostart: !has("--no-autostart"),
            start_menu: !has("--no-start-menu"),
            browsers: !has("--no-browsers"),
            open_after: has("--open"),
            delete_data: has("--delete-data"),
        };
        let result = if removing { uninstall(choices, &mut |_, _| {}) } else { install(choices, &mut |_, _| {}) };
        finish_temp_copy(&args);
        match result {
            Ok(()) => {
                log(if removing { "removed" } else { "installed" });
                std::process::exit(0)
            }
            Err(e) => {
                log(&format!("failed: {e:#}"));
                std::process::exit(1)
            }
        }
    }

    if let Err(e) = run_window(removing) {
        log(&format!("window failed: {e:#}"));
    }
    finish_temp_copy(&args);
}

const MOVED_PREFIX: &str = "sdm-remove-old-";

/// Files moved out of the app folder because a process still held them.
static MOVED_OUT: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

fn finish_temp_copy(args: &[String]) {
    let mut files = std::mem::take(&mut *MOVED_OUT.lock().unwrap());
    if args.iter().any(|a| a == "--from-temp")
        && let Ok(me) = std::env::current_exe()
    {
        files.push(me);
    }
    sys::delete_later(&files);
}

fn run_window(removing: bool) -> Result<()> {
    let ui = SetupWindow::new()?;
    ui.set_version(VERSION.into());
    ui.set_installed_version(sys::installed_version().unwrap_or_default().into());
    ui.set_removing(removing);
    ui.set_page(if removing { "remove" } else { "welcome" }.into());
    ui.set_folder(bin_dir().display().to_string().into());
    ui.set_autostart(true);

    let weak = ui.as_weak();
    ui.on_install(move || start_work(&weak, false));
    let weak = ui.as_weak();
    ui.on_remove(move || start_work(&weak, true));
    let weak = ui.as_weak();
    ui.on_finish(move || {
        if let Some(ui) = weak.upgrade() {
            let _ = ui.hide();
        }
        let _ = slint::quit_event_loop();
    });
    ui.on_cancel(|| {
        let _ = slint::quit_event_loop();
    });

    ui.show()?;
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(30), move || {
        if let Some(ui) = weak.upgrade() {
            sys::apply_glass(ui.window());
        }
    });
    slint::run_event_loop()?;
    Ok(())
}

fn start_work(weak: &slint::Weak<SetupWindow>, removing: bool) {
    let Some(ui) = weak.upgrade() else { return };
    let choices = Choices {
        autostart: ui.get_autostart(),
        start_menu: ui.get_start_menu(),
        browsers: ui.get_browsers(),
        open_after: ui.get_open_after(),
        delete_data: ui.get_delete_data(),
    };
    ui.set_page("working".into());
    ui.set_progress(0.0);
    ui.set_step(if removing { "Removing" } else { "Installing" }.into());
    let weak = weak.clone();
    std::thread::spawn(move || {
        let report_weak = weak.clone();
        let mut report = move |step: &str, progress: f32| {
            let step = step.to_string();
            let w = report_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = w.upgrade() {
                    ui.set_step(step.into());
                    ui.set_progress(progress);
                }
            });
        };
        let result = if removing { uninstall(choices, &mut report) } else { install(choices, &mut report) };
        if let Err(e) = &result {
            log(&format!("failed: {e:#}"));
        }
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                match result {
                    Ok(()) => ui.set_page("done".into()),
                    Err(e) => {
                        ui.set_failure(format!("{e:#}").into());
                        ui.set_page("failed".into());
                    }
                }
            }
        });
    });
}

type Report<'a> = &'a mut dyn FnMut(&str, f32);

fn install(c: Choices, report: Report) -> Result<()> {
    if APP_BYTES.is_empty() || HOST_BYTES.is_empty() {
        bail!("This installer was built without the app inside. Download it again from the releases page.");
    }
    let bin = bin_dir();
    fs::create_dir_all(&bin).with_context(|| format!("could not create {}", bin.display()))?;
    clear_old_files(&bin);

    report("Closing the running copy", 0.1);
    let was_running = stop_running_copy();

    report("Copying the app", 0.3);
    let app = bin.join(APP_EXE);
    let host = bin.join(HOST_EXE);
    place(&app, APP_BYTES)?;
    place(&host, HOST_BYTES)?;
    let me = std::env::current_exe().context("could not find the installer itself")?;
    let uninstaller = bin.join(UNINSTALL_EXE);
    if !same_file(&me, &uninstaller) {
        let bytes = fs::read(&me).context("could not read the installer")?;
        place(&uninstaller, &bytes)?;
    }

    report("Connecting your browsers", 0.55);
    if c.browsers {
        sys::run_hidden(&host, &["--install"]).context("could not connect the browsers")?;
    }

    report("Adding shortcuts", 0.7);
    let shortcut = sys::start_menu_shortcut()?;
    if c.start_menu {
        sys::make_shortcut(&shortcut, &app)?;
    } else {
        let _ = fs::remove_file(&shortcut);
    }
    sys::set_autostart(c.autostart.then_some(app.as_path()))?;
    let size_kb = ((APP_BYTES.len() + HOST_BYTES.len() + fs::metadata(&uninstaller).map(|m| m.len() as usize).unwrap_or(0)) / 1024) as u32;
    sys::register_uninstall(&bin, &app, &uninstaller, VERSION, size_kb)?;

    report("Done", 1.0);
    if c.open_after {
        sys::launch(&app, &[])?;
    } else if was_running {
        sys::launch(&app, &["--background"])?;
    }
    Ok(())
}

fn uninstall(c: Choices, report: Report) -> Result<()> {
    let bin = bin_dir();
    report("Closing the running copy", 0.1);
    stop_running_copy();

    report("Disconnecting your browsers", 0.3);
    let host = bin.join(HOST_EXE);
    if host.is_file() {
        sys::run_hidden(&host, &["--uninstall"]).context("could not disconnect the browsers")?;
    }

    report("Removing shortcuts", 0.5);
    if let Ok(shortcut) = sys::start_menu_shortcut() {
        let _ = fs::remove_file(shortcut);
    }
    sys::set_autostart(None)?;
    sys::unregister_uninstall()?;

    report("Removing the app", 0.7);
    remove_with_retries(&bin)?;
    if c.delete_data {
        // The Windhawk mod keeps its engine in `engine`; it may be loaded, and it is not ours.
        let dir = data_dir();
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name == "engine" || name.starts_with(MOVED_PREFIX) {
                    continue;
                }
                let p = e.path();
                let r = if p.is_dir() { fs::remove_dir_all(&p) } else { fs::remove_file(&p) };
                r.with_context(|| format!("could not delete {}", p.display()))?;
            }
        }
        let _ = fs::remove_dir(&dir);
    }
    report("Done", 1.0);
    Ok(())
}

/// Asks a running app (or the Windhawk mod's engine) to close, then waits for it to let go.
/// True when something was running.
fn stop_running_copy() -> bool {
    if !sys::instance_running() {
        return false;
    }
    if let Some((port, key)) = read_api(&data_dir().join("settings.json")) {
        let _ = post_exit(port, &key);
    }
    let until = Instant::now() + Duration::from_secs(10);
    while sys::instance_running() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(200));
    }
    // The process may hold its files a moment after the lock goes.
    std::thread::sleep(Duration::from_millis(400));
    true
}

fn read_api(settings: &Path) -> Option<(u16, String)> {
    let text = fs::read_to_string(settings).ok()?;
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let port = v.get("api_port")?.as_u64()? as u16;
    let key = v.get("api_key")?.as_str()?.to_string();
    Some((port, key))
}

fn post_exit(port: u16, key: &str) -> std::io::Result<()> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    s.set_read_timeout(Some(Duration::from_secs(3)))?;
    let req = format!(
        "POST /v1/exit HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Api-Key: {key}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
    );
    s.write_all(req.as_bytes())?;
    let mut buf = [0u8; 256];
    let _ = s.read(&mut buf);
    Ok(())
}

/// Writes a program file. A file Windows still holds open can be renamed out of the way.
fn place(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("new");
    fs::write(&tmp, bytes).with_context(|| format!("could not write {}", tmp.display()))?;
    if fs::rename(&tmp, path).is_ok() {
        return Ok(());
    }
    let old = path.with_extension(format!("{}.old", std::process::id()));
    fs::rename(path, &old).with_context(|| format!("{} is in use and could not be replaced", path.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("could not put {} in place", path.display()))?;
    Ok(())
}

fn clear_old_files(bin: &Path) {
    if let Ok(entries) = fs::read_dir(bin) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".old") || name.ends_with(".new") {
                let _ = fs::remove_file(e.path());
            }
        }
    }
}

fn remove_with_retries(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let mut last = None;
    for attempt in 0..10 {
        match fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        // A running program can be renamed but not deleted. The remover the caller
        // started waits on this copy, so move it out and delete it once it exits.
        if attempt >= 1 {
            move_out_locked(dir);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(last.unwrap()).with_context(|| format!("could not delete {}", dir.display()))
}

fn move_out_locked(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() || fs::remove_file(&p).is_ok() {
            continue;
        }
        let n = MOVED_OUT.lock().unwrap().len();
        let name = format!("{MOVED_PREFIX}{}-{n}.exe", std::process::id());
        // The temp folder can sit on another drive; beside the app folder never does.
        let places = [std::env::temp_dir(), dir.parent().map(Path::to_path_buf).unwrap_or_default()];
        for to in places.iter().map(|d| d.join(&name)) {
            if fs::rename(&p, &to).is_ok() {
                MOVED_OUT.lock().unwrap().push(to);
                break;
            }
        }
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_port_and_key() {
        let dir = std::env::temp_dir().join(format!("sdm-setup-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        fs::write(&p, "\u{feff}{\"api_port\":15161,\"api_key\":\"abc\",\"other\":1}").unwrap();
        assert_eq!(read_api(&p), Some((15161, "abc".to_string())));
        fs::write(&p, "{}").unwrap();
        assert_eq!(read_api(&p), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn place_replaces_a_file() {
        let dir = std::env::temp_dir().join(format!("sdm-setup-place-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.exe");
        place(&p, b"one").unwrap();
        place(&p, b"two").unwrap();
        assert_eq!(fs::read(&p).unwrap(), b"two");
        fs::remove_dir_all(&dir).unwrap();
    }
}
