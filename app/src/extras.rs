//! The extras beside the main list: the power step after a queue, sounds, the update check,
//! checking a finished file, adding many links from one pattern, name lookup and site rules.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sdm_core::checksum::{self, HashKind};
use sdm_core::util::{BatchPadding, expand_batch};
use sdm_core::{AddRequest, PerHostRule, PowerAction, Settings};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{AppWindow, BatchBox, PowerBox, Prefs, SiteBox, SiteView, Sites, SumBox, Update, View};

thread_local! {
    static VIEW: RefCell<Option<Rc<RefCell<View>>>> = const { RefCell::new(None) };
    static POWER: RefCell<Option<(slint::Timer, PowerAction)>> = const { RefCell::new(None) };
    static UPDATE_CHECKED: Cell<bool> = const { Cell::new(false) };
    static SUM_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static SUM_GEN: Cell<u64> = const { Cell::new(0) };
}

/// A toast from a callback that has only the window, run on the window thread.
fn toast(app: &AppWindow, text: impl Into<SharedString>, tone: i32) {
    if let Some(view) = VIEW.with(|v| v.borrow().clone()) {
        crate::toast(app, &view, text, tone);
    }
}

fn manager() -> Option<std::sync::Arc<sdm_core::Manager>> {
    VIEW.with(|v| v.borrow().as_ref().map(|view| view.borrow().manager.clone()))
}

pub fn wire(app: &AppWindow, view: &Rc<RefCell<View>>) {
    VIEW.with(|v| *v.borrow_mut() = Some(view.clone()));
    let weak = app.as_weak();

    let power = app.global::<PowerBox>();
    {
        let weak = weak.clone();
        power.on_cancel(move || {
            if let Some(app) = weak.upgrade() {
                cancel_power(&app);
            }
        });
    }
    {
        let weak = weak.clone();
        power.on_now(move || {
            if let Some(app) = weak.upgrade() {
                finish_power(&app);
            }
        });
    }

    {
        let weak = weak.clone();
        app.global::<Update>().on_check(move || {
            if let Some(app) = weak.upgrade() {
                check_update(&app);
            }
        });
    }

    let sum = app.global::<SumBox>();
    {
        let weak = weak.clone();
        sum.on_kind_picked(move |_| {
            if let Some(app) = weak.upgrade() {
                run_sum(&app);
            }
        });
    }
    {
        let weak = weak.clone();
        sum.on_expected_edited(move |text| {
            let Some(app) = weak.upgrade() else { return };
            let s = app.global::<SumBox>();
            let guess = HashKind::from_hex_len(checksum::normalize(&text).len()).and_then(|k| HashKind::ALL.iter().position(|x| *x == k));
            match guess {
                Some(i) if i as i32 != s.get_kind() => {
                    s.set_kind(i as i32);
                    run_sum(&app);
                }
                _ => update_verdict(&s),
            }
        });
    }
    {
        let weak = weak.clone();
        sum.on_copy(move || {
            let Some(app) = weak.upgrade() else { return };
            let hex = app.global::<SumBox>().get_result();
            if !hex.is_empty() && crate::set_clipboard(&hex) {
                toast(&app, "Copied", 1);
            }
        });
    }

    let batch = app.global::<BatchBox>();
    {
        let weak = weak.clone();
        batch.on_changed(move || {
            if let Some(app) = weak.upgrade() {
                batch_preview(&app);
            }
        });
    }
    {
        let weak = weak.clone();
        batch.on_browse(move || {
            let Some(app) = weak.upgrade() else { return };
            let b = app.global::<BatchBox>();
            if let Some(dir) = crate::pick_folder(&b.get_folder()) {
                b.set_folder(dir.into());
            }
        });
    }
    {
        let weak = weak.clone();
        batch.on_add(move |start_queue| {
            if let Some(app) = weak.upgrade() {
                batch_add(&app, start_queue);
            }
        });
    }

    let sites = app.global::<Sites>();
    {
        let weak = weak.clone();
        sites.on_add(move || {
            if let Some(app) = weak.upgrade() {
                open_site(&app, None);
            }
        });
    }
    {
        let weak = weak.clone();
        sites.on_edit(move |i| {
            if let Some(app) = weak.upgrade() {
                open_site(&app, Some(i.max(0) as usize));
            }
        });
    }
    {
        let weak = weak.clone();
        sites.on_delete(move |i| {
            let (Some(app), Some(manager)) = (weak.upgrade(), manager()) else { return };
            let mut s = manager.settings();
            let i = i.max(0) as usize;
            if i >= s.per_host.len() {
                return;
            }
            let gone = s.per_host.remove(i);
            match manager.update_settings(s) {
                Ok(()) => {
                    load_sites(&app, &manager.settings());
                    toast(&app, format!("Removed the rule for {}", gone.host), 0);
                }
                Err(e) => toast(&app, format!("Could not save: {e}"), 4),
            }
        });
    }
    {
        let weak = weak.clone();
        app.global::<SiteBox>().on_save(move || {
            if let Some(app) = weak.upgrade() {
                save_site(&app);
            }
        });
    }

    let prefs = app.global::<Prefs>();
    {
        let weak = weak.clone();
        prefs.on_choose_sound(move |done| {
            let Some(app) = weak.upgrade() else { return };
            let Some(file) = rfd::FileDialog::new().set_title("Choose a sound").add_filter("Sound", &["wav"]).pick_file() else { return };
            let path = file.display().to_string();
            set_sound(&app.global::<Prefs>(), done, &path);
            app.global::<Prefs>().set_dirty(true);
            play(&path, done);
        });
    }
    {
        let weak = weak.clone();
        prefs.on_clear_sound(move |done| {
            let Some(app) = weak.upgrade() else { return };
            set_sound(&app.global::<Prefs>(), done, "");
            app.global::<Prefs>().set_dirty(true);
        });
    }
}

// ---------- power step after a queue ----------

const POWER_SECONDS: i32 = 60;

/// What the computer is asked to do. Kept apart so it can be tested without doing it.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Nothing,
    Shutdown(&'static [&'static str]),
    Suspend { hibernate: bool },
}

pub fn step_for(action: PowerAction) -> Step {
    match action {
        PowerAction::None => Step::Nothing,
        PowerAction::Sleep => Step::Suspend { hibernate: false },
        PowerAction::Hibernate => Step::Suspend { hibernate: true },
        PowerAction::Shutdown => Step::Shutdown(&["/s", "/t", "0"]),
        PowerAction::Restart => Step::Shutdown(&["/r", "/t", "0"]),
    }
}

fn words(action: PowerAction) -> (&'static str, &'static str) {
    match action {
        PowerAction::Sleep => ("go to sleep", "Sleep now"),
        PowerAction::Hibernate => ("hibernate", "Hibernate now"),
        PowerAction::Shutdown => ("shut down", "Shut down now"),
        PowerAction::Restart => ("restart", "Restart now"),
        PowerAction::None => ("", ""),
    }
}

/// A queue finished: count down a minute with a way out, then do the step.
pub fn start_power(app: &AppWindow, queue: &str, action: PowerAction) {
    if step_for(action) == Step::Nothing {
        return;
    }
    take_power();
    let (what, button) = words(action);
    let p = app.global::<PowerBox>();
    p.set_title(format!("{queue} is done").into());
    p.set_what(what.into());
    p.set_button(button.into());
    p.set_seconds(POWER_SECONDS);
    p.set_total(POWER_SECONDS);
    p.set_open(true);
    if let Some(view) = VIEW.with(|v| v.borrow().clone()) {
        crate::show_window_quiet(app, &view);
    }
    let timer = slint::Timer::default();
    let weak = app.as_weak();
    timer.start(slint::TimerMode::Repeated, Duration::from_secs(1), move || {
        let Some(app) = weak.upgrade() else { return };
        let p = app.global::<PowerBox>();
        let left = (p.get_seconds() - 1).max(0);
        p.set_seconds(left);
        if left == 0 {
            finish_power(&app);
        }
    });
    POWER.with(|slot| *slot.borrow_mut() = Some((timer, action)));
}

fn take_power() -> Option<PowerAction> {
    let taken = POWER.with(|slot| slot.borrow_mut().take());
    taken.map(|(timer, action)| {
        timer.stop();
        // Dropped on the next turn, never inside its own tick.
        slint::Timer::single_shot(Duration::ZERO, move || drop(timer));
        action
    })
}

fn cancel_power(app: &AppWindow) {
    if take_power().is_some() {
        toast(app, "Cancelled. The computer stays on.", 0);
    }
    app.global::<PowerBox>().set_open(false);
}

fn finish_power(app: &AppWindow) {
    app.global::<PowerBox>().set_open(false);
    let Some(action) = take_power() else { return };
    if let Err(e) = run_power(action) {
        toast(app, format!("Could not {}: {e}", words(action).0), 4);
    }
}

fn run_power(action: PowerAction) -> Result<(), String> {
    // For tests: write down what would happen and do nothing.
    if std::env::var_os("SDM_POWER_DRY_RUN").is_some() {
        let note = crate::data_dir().join("power-dry-run.txt");
        return std::fs::write(note, format!("{}\n", action.as_str())).map_err(|e| e.to_string());
    }
    match step_for(action) {
        Step::Nothing => Ok(()),
        Step::Shutdown(args) => {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("shutdown")
                .args(args)
                .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        Step::Suspend { hibernate } => {
            let ok = unsafe { windows::Win32::System::Power::SetSuspendState(hibernate, false, false) };
            if ok { Ok(()) } else { Err("Windows said no".into()) }
        }
    }
}

// ---------- sounds ----------

static LAST_SOUND: AtomicU64 = AtomicU64::new(0);

/// A download finished or failed. At most one sound every two seconds, so a big batch is quiet.
pub fn play_done(settings: &Settings, ok: bool) {
    if !settings.notification_sound {
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let last = LAST_SOUND.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 2000 || LAST_SOUND.compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed).is_err() {
        return;
    }
    play(if ok { &settings.success_notification_sound } else { &settings.error_notification_sound }, ok);
}

/// A chosen .wav, or the Windows sound for the event when none is chosen.
fn play(file: &str, ok: bool) {
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
    use windows::core::HSTRING;
    let file = file.trim();
    unsafe {
        if !file.is_empty() && Path::new(file).is_file() {
            let _ = PlaySoundW(&HSTRING::from(file), None, SND_FILENAME | SND_ASYNC | SND_NODEFAULT);
        } else {
            let alias = if ok { "SystemAsterisk" } else { "SystemHand" };
            let _ = PlaySoundW(&HSTRING::from(alias), None, SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
        }
    }
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn set_sound(p: &Prefs, done: bool, path: &str) {
    let shown: SharedString = if path.is_empty() { "The Windows sound".into() } else { file_name(path).into() };
    if done {
        p.set_sound_done(path.into());
        p.set_sound_done_name(shown);
    } else {
        p.set_sound_fail(path.into());
        p.set_sound_fail_name(shown);
    }
}

// ---------- name lookup ----------

pub const DNS: [&str; 4] = ["", "https://cloudflare-dns.com/dns-query", "https://dns.google/dns-query", "https://dns.quad9.net/dns-query"];
const DNS_CUSTOM: i32 = 4;

/// The dropdown place for a saved address, and the text for the custom field.
pub fn dns_choice(url: &str) -> (i32, String) {
    let url = url.trim();
    match DNS.iter().position(|d| d.eq_ignore_ascii_case(url)) {
        Some(i) => (i as i32, String::new()),
        None => (DNS_CUSTOM, url.to_string()),
    }
}

pub fn dns_url(choice: i32, custom: &str) -> Result<String, String> {
    if (0..DNS_CUSTOM).contains(&choice) {
        return Ok(DNS[choice as usize].to_string());
    }
    let custom = custom.trim();
    match url::Url::parse(custom) {
        Ok(u) if u.scheme() == "https" && u.host_str().is_some() => Ok(custom.to_string()),
        _ => Err("The name lookup address must start with https://".into()),
    }
}

/// The extra settings, read into the settings page.
pub fn load_prefs(app: &AppWindow, s: &Settings) {
    let p = app.global::<Prefs>();
    let (choice, custom) = dns_choice(&s.dns_over_https);
    p.set_dns_choice(choice);
    p.set_dns_url(custom.into());
    p.set_sound_on(s.notification_sound);
    set_sound(&p, true, &s.success_notification_sound);
    set_sound(&p, false, &s.error_notification_sound);
    load_sites(app, s);
}

pub fn save_prefs(app: &AppWindow, s: &mut Settings) -> Result<(), String> {
    let p = app.global::<Prefs>();
    s.dns_over_https = dns_url(p.get_dns_choice(), &p.get_dns_url())?;
    s.notification_sound = p.get_sound_on();
    s.success_notification_sound = p.get_sound_done().to_string();
    s.error_notification_sound = p.get_sound_fail().to_string();
    Ok(())
}

// ---------- update check ----------

const RELEASES: &str = "https://api.github.com/repos/DavidHiFi/seamless-download-manager/releases/latest";

/// True when `tag` (like v1.4.0) is a later version than `current`.
pub fn is_newer(tag: &str, current: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.trim().trim_start_matches(['v', 'V']).split(|c: char| !c.is_ascii_digit()).filter(|p| !p.is_empty()).take(3).map(|p| p.parse().unwrap_or(0)).collect()
    }
    let (a, b) = (parts(tag), parts(current));
    if a.is_empty() {
        return false;
    }
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// The first visit to the About page checks once.
pub fn check_update_once(app: &AppWindow) {
    if !UPDATE_CHECKED.with(|c| c.replace(true)) {
        check_update(app);
    }
}

fn check_update(app: &AppWindow) {
    UPDATE_CHECKED.with(|c| c.set(true));
    let u = app.global::<Update>();
    if u.get_checking() {
        return;
    }
    u.set_checking(true);
    u.set_tone(0);
    u.set_status("Looking for a new version".into());
    let weak = app.as_weak();
    tokio::spawn(async move {
        let result = latest_release().await;
        let _ = weak.upgrade_in_event_loop(move |app| {
            let u = app.global::<Update>();
            u.set_checking(false);
            let (text, tone, url) = match result {
                Ok(Some((tag, url))) if is_newer(&tag, env!("CARGO_PKG_VERSION")) => {
                    (format!("Version {} is out.", tag.trim_start_matches(['v', 'V'])), 2, url)
                }
                Ok(Some(_)) => ("You have the newest version.".to_string(), 1, String::new()),
                Ok(None) => ("No public release yet.".to_string(), 0, String::new()),
                Err(e) => (format!("Could not check: {e}"), 4, String::new()),
            };
            u.set_has_update(!url.is_empty());
            u.set_url(url.into());
            u.set_status(text.into());
            u.set_tone(tone);
        });
    });
}

async fn latest_release() -> Result<Option<(String, String)>, String> {
    let client = reqwest::Client::builder()
        .user_agent("SeamlessDownloadManager")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| if e.is_timeout() { "GitHub did not answer in time".to_string() } else { "GitHub could not be reached".to_string() })?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(format!("GitHub answered {}", resp.status().as_u16()));
    }
    let v: serde_json::Value = resp.json().await.map_err(|_| "GitHub sent something unexpected".to_string())?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    let url = v.get("html_url").and_then(|t| t.as_str()).unwrap_or_default().to_string();
    Ok(Some((tag, url)))
}

// ---------- checking a finished file ----------

pub fn open_sum(app: &AppWindow, path: PathBuf) {
    let s = app.global::<SumBox>();
    s.set_file(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into());
    let kinds: Vec<SharedString> = HashKind::ALL.iter().map(|k| SharedString::from(k.label())).collect();
    s.set_kinds(ModelRc::new(VecModel::from(kinds)));
    s.set_kind(HashKind::ALL.iter().position(|k| *k == HashKind::Sha256).unwrap_or(0) as i32);
    s.set_expected(SharedString::new());
    SUM_PATH.with(|p| *p.borrow_mut() = Some(path));
    s.set_open(true);
    run_sum(app);
}

fn run_sum(app: &AppWindow) {
    let s = app.global::<SumBox>();
    let Some(path) = SUM_PATH.with(|p| p.borrow().clone()) else { return };
    let kind = HashKind::ALL[(s.get_kind().max(0) as usize).min(HashKind::ALL.len() - 1)];
    let generation = SUM_GEN.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    s.set_busy(true);
    s.set_result(SharedString::new());
    s.set_verdict("Reading the file".into());
    s.set_tone(0);
    let weak = app.as_weak();
    tokio::task::spawn_blocking(move || {
        let result = checksum::hash_file(kind, &path).map_err(|e| e.to_string());
        let _ = weak.upgrade_in_event_loop(move |app| {
            if SUM_GEN.with(Cell::get) != generation {
                return;
            }
            let s = app.global::<SumBox>();
            s.set_busy(false);
            match result {
                Ok(hex) => {
                    s.set_result(hex.into());
                    update_verdict(&s);
                }
                Err(e) => {
                    s.set_verdict(format!("Could not read the file: {e}").into());
                    s.set_tone(4);
                }
            }
        });
    });
}

fn update_verdict(s: &SumBox) {
    let hex = s.get_result().to_string();
    if hex.is_empty() {
        return;
    }
    let expected = checksum::normalize(&s.get_expected());
    let (text, tone) = if expected.is_empty() {
        ("Paste the value from the download page to compare.", 0)
    } else if expected.eq_ignore_ascii_case(&hex) {
        ("Matches. The file is exactly what the site sent.", 1)
    } else {
        ("Does not match. The file may be damaged or changed.", 4)
    };
    s.set_verdict(text.into());
    s.set_tone(tone);
}

// ---------- adding many links from one pattern ----------

pub fn open_batch(app: &AppWindow) {
    let Some(manager) = manager() else { return };
    let b = app.global::<BatchBox>();
    let clip = crate::get_clipboard()
        .map(|c| c.trim().to_string())
        .filter(|c| c.contains('*') && (c.starts_with("http://") || c.starts_with("https://")) && !c.contains(char::is_whitespace))
        .unwrap_or_default();
    b.set_pattern(clip.into());
    b.set_start("1".into());
    b.set_end("10".into());
    b.set_padding(0);
    b.set_width("3".into());
    b.set_folder(manager.settings().default_download_folder.into());
    b.set_queue_index(0);
    b.set_open(true);
    batch_preview(app);
}

fn batch_links(b: &BatchBox) -> Result<Vec<String>, String> {
    let start = b.get_start().trim().parse::<u64>().map_err(|_| "The first number must be a whole number.".to_string())?;
    let end = b.get_end().trim().parse::<u64>().map_err(|_| "The last number must be a whole number.".to_string())?;
    let padding = match b.get_padding() {
        1 => BatchPadding::None,
        2 => BatchPadding::Width(b.get_width().trim().parse::<usize>().map_err(|_| "The width must be a whole number.".to_string())?.min(20)),
        _ => BatchPadding::Auto,
    };
    expand_batch(&b.get_pattern(), start, end, padding)
}

fn batch_preview(app: &AppWindow) {
    let b = app.global::<BatchBox>();
    match batch_links(&b) {
        Ok(links) => {
            b.set_count(if links.len() == 1 { "1 link".into() } else { format!("{} links", links.len()).into() });
            b.set_first(links.first().cloned().unwrap_or_default().into());
            b.set_last(if links.len() > 1 { links.last().cloned().unwrap_or_default().into() } else { SharedString::new() });
            b.set_error(SharedString::new());
            b.set_can_add(true);
        }
        Err(e) => {
            b.set_count(SharedString::new());
            b.set_first(SharedString::new());
            b.set_last(SharedString::new());
            b.set_error(if b.get_pattern().trim().is_empty() { SharedString::new() } else { e.into() });
            b.set_can_add(false);
        }
    }
}

fn batch_add(app: &AppWindow, start_queue: bool) {
    let b = app.global::<BatchBox>();
    let Ok(links) = batch_links(&b) else { return };
    let Some(view) = VIEW.with(|v| v.borrow().clone()) else { return };
    let (manager, qid) = {
        let v = view.borrow();
        (v.manager.clone(), v.queue_ids.get(b.get_queue_index().max(0) as usize).copied().unwrap_or(0))
    };
    let folder = Some(b.get_folder().to_string()).filter(|f| !f.trim().is_empty());
    b.set_can_add(false);
    let weak = app.as_weak();
    // Up to a thousand rows: off the window thread so it stays smooth.
    tokio::task::spawn_blocking(move || {
        let total = links.len();
        let mut added = 0usize;
        let mut failed = None;
        for url in links {
            match manager.add(AddRequest { url, folder: folder.clone(), queue_id: Some(qid), ..Default::default() }) {
                Ok(_) => added += 1,
                Err(e) => {
                    failed = Some(e.to_string());
                    break;
                }
            }
        }
        if start_queue && added > 0 {
            manager.start_queue(qid);
        }
        let _ = weak.upgrade_in_event_loop(move |app| {
            let b = app.global::<BatchBox>();
            match failed {
                None => {
                    b.set_open(false);
                    let what = if total == 1 { "1 link".to_string() } else { format!("{total} links") };
                    toast(&app, if start_queue { format!("Added {what} and started the queue") } else { format!("Added {what} to the queue") }, 1);
                    app.invoke_focus_list();
                }
                Some(e) => {
                    b.set_error(format!("Added {added} of {total}, then: {e}").into());
                    b.set_can_add(true);
                }
            }
        });
    });
}

// ---------- site rules ----------

pub fn site_summary(r: &PerHostRule) -> String {
    let mut parts = Vec::new();
    if !r.username.is_empty() {
        parts.push("Signs in".to_string());
    }
    if !r.user_agent.trim().is_empty() {
        parts.push("Own browser name".to_string());
    }
    if r.threads > 0 {
        parts.push(format!("{} connection{}", r.threads, if r.threads == 1 { "" } else { "s" }));
    }
    if r.speed_limit > 0 {
        parts.push(format!("{} KB/s", (r.speed_limit / 1024).max(1)));
    }
    if parts.is_empty() { "Uses the normal settings".to_string() } else { parts.join(", ") }
}

pub fn load_sites(app: &AppWindow, s: &Settings) {
    let list: Vec<SiteView> =
        s.per_host.iter().map(|r| SiteView { host: r.host.as_str().into(), summary: site_summary(r).into() }).collect();
    app.global::<Sites>().set_rules(ModelRc::new(VecModel::from(list)));
}

thread_local! {
    static SITE_INDEX: Cell<Option<usize>> = const { Cell::new(None) };
}

fn open_site(app: &AppWindow, index: Option<usize>) {
    let Some(manager) = manager() else { return };
    let rule = index.and_then(|i| manager.settings().per_host.get(i).cloned());
    let index = if rule.is_some() { index } else { None };
    let rule = rule.unwrap_or_default();
    SITE_INDEX.with(|c| c.set(index));
    let d = app.global::<SiteBox>();
    d.set_is_new(index.is_none());
    d.set_host(rule.host.into());
    d.set_user(rule.username.into());
    d.set_pass(rule.password.into());
    d.set_agent(rule.user_agent.into());
    d.set_threads(if rule.threads > 0 { rule.threads.to_string().into() } else { SharedString::new() });
    d.set_kb(if rule.speed_limit > 0 { (rule.speed_limit / 1024).max(1).to_string().into() } else { SharedString::new() });
    d.set_error(SharedString::new());
    d.set_open(true);
}

/// `https://Files.Example.com/a` and `files.example.com` are the same site.
pub fn clean_host(text: &str) -> Result<String, String> {
    let t = text.trim();
    let t = t.strip_prefix("https://").or_else(|| t.strip_prefix("http://")).unwrap_or(t);
    let host = t.split(['/', '?', '#']).next().unwrap_or_default().to_ascii_lowercase();
    if host.is_empty() || host.contains(char::is_whitespace) {
        return Err("Type the site, like files.example.com or *.example.com.".into());
    }
    Ok(host)
}

fn optional_num(text: &str, what: &str, max: u64) -> Result<u64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(0);
    }
    match t.parse::<u64>() {
        Ok(n) if n <= max => Ok(n),
        _ => Err(format!("{what} must be a number from 0 to {max}, or empty.")),
    }
}

fn save_site(app: &AppWindow) {
    let Some(manager) = manager() else { return };
    let d = app.global::<SiteBox>();
    let rule = (|| -> Result<PerHostRule, String> {
        Ok(PerHostRule {
            host: clean_host(&d.get_host())?,
            username: d.get_user().trim().to_string(),
            password: d.get_pass().to_string(),
            user_agent: d.get_agent().trim().to_string(),
            threads: optional_num(&d.get_threads(), "Connections", 32)? as u32,
            speed_limit: optional_num(&d.get_kb(), "The speed limit", 10_000_000)? * 1024,
        })
    })();
    let rule = match rule {
        Ok(r) => r,
        Err(e) => {
            d.set_error(e.into());
            return;
        }
    };
    let mut s = manager.settings();
    let host = rule.host.clone();
    match SITE_INDEX.with(Cell::get).filter(|i| *i < s.per_host.len()) {
        Some(i) => s.per_host[i] = rule,
        None => s.per_host.push(rule),
    }
    match manager.update_settings(s) {
        Ok(()) => {
            d.set_open(false);
            load_sites(app, &manager.settings());
            toast(app, format!("Saved the rule for {host}"), 1);
        }
        Err(e) => d.set_error(format!("Could not save: {e}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_steps() {
        assert_eq!(step_for(PowerAction::None), Step::Nothing);
        assert_eq!(step_for(PowerAction::Sleep), Step::Suspend { hibernate: false });
        assert_eq!(step_for(PowerAction::Hibernate), Step::Suspend { hibernate: true });
        assert_eq!(step_for(PowerAction::Shutdown), Step::Shutdown(&["/s", "/t", "0"]));
        assert_eq!(step_for(PowerAction::Restart), Step::Shutdown(&["/r", "/t", "0"]));
    }

    #[test]
    fn versions_compare() {
        assert!(is_newer("v1.2.0", "1.1.9"));
        assert!(is_newer("2.0", "1.9.9"));
        assert!(!is_newer("v1.1.0", "1.1.0"));
        assert!(!is_newer("1.0.9", "1.1.0"));
        assert!(!is_newer("nightly", "0.1.0"));
        assert!(is_newer("v0.2.0-beta", "0.1.0"));
    }

    #[test]
    fn dns_round_trip() {
        assert_eq!(dns_choice(""), (0, String::new()));
        assert_eq!(dns_choice("https://dns.google/dns-query"), (2, String::new()));
        assert_eq!(dns_choice("https://my.dns/q"), (DNS_CUSTOM, "https://my.dns/q".into()));
        assert_eq!(dns_url(1, "").unwrap(), DNS[1]);
        assert_eq!(dns_url(0, "x").unwrap(), "");
        assert_eq!(dns_url(DNS_CUSTOM, " https://my.dns/q ").unwrap(), "https://my.dns/q");
        assert!(dns_url(DNS_CUSTOM, "http://plain.dns/q").is_err());
        assert!(dns_url(DNS_CUSTOM, "").is_err());
    }

    #[test]
    fn hosts_and_summaries() {
        assert_eq!(clean_host(" https://Files.Example.com/a?b ").unwrap(), "files.example.com");
        assert_eq!(clean_host("*.example.com").unwrap(), "*.example.com");
        assert!(clean_host("  ").is_err());
        assert!(clean_host("a b.com").is_err());
        let r = PerHostRule { host: "x".into(), username: "u".into(), threads: 1, speed_limit: 512 * 1024, ..Default::default() };
        assert_eq!(site_summary(&r), "Signs in, 1 connection, 512 KB/s");
        assert_eq!(site_summary(&PerHostRule::default()), "Uses the normal settings");
        assert_eq!(optional_num("", "x", 5), Ok(0));
        assert!(optional_num("6", "x", 5).is_err());
    }
}
