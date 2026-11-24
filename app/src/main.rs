#![windows_subsystem = "windows"]

mod cli;
mod extras;
mod fmt;
mod tray;
mod win;

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use sdm_core::settings::{SpeedLimitMode, UnitMode};
use sdm_core::{AddRequest, Category, DownloadView, Manager, ManagerEvent, PowerAction, Queue, Status, Weekday};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

slint::include_modules!();

const LIST_DIRTY: u8 = 1;
const QUEUES_DIRTY: u8 = 2;
const SETTINGS_DIRTY: u8 = 4;

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("SeamlessDownloadManager")
}

/// What the window keeps between refreshes. Lives on the window thread only.
struct View {
    manager: Arc<Manager>,
    shown: Vec<DownloadView>,
    selected: HashSet<i64>,
    anchor: Option<usize>,
    rows: Rc<VecModel<Row>>,
    pending_remove: Vec<i64>,
    pending_limit: Option<i64>,
    pending_add: Option<AddRequest>,
    auto_name: String,
    queue_ids: Vec<i64>,
    glass_done: bool,
    toast_gen: u64,
}

/// Flags and hand-offs written by the background threads, read by the window timer.
struct Shared {
    dirty: std::sync::atomic::AtomicU8,
    browser_add: Mutex<Option<AddRequest>>,
    probe_gen: AtomicU64,
    quitting: AtomicBool,
    /// A queue that ended with a power step, picked up by the window timer.
    power: Mutex<Option<(String, PowerAction)>>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = cli::run(&args, &data_dir()) {
        std::process::exit(code);
    }
    let background = std::env::args().any(|a| a == "--background");
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build()?;
    let _guard = rt.enter();
    let dir = data_dir();

    let Some(_instance) = win::claim_instance() else {
        rt.block_on(wake_running_copy(&dir));
        return Ok(());
    };

    let manager = Manager::open(dir.clone())?;
    let api = rt.block_on(sdm_api::serve(manager.clone()));
    let (api_up, api_text) = match &api {
        Ok(handle) => (true, format!("Listening on port {}.", handle.port())),
        Err(e) => (false, format!("Could not start: {e}")),
    };

    let app = AppWindow::new()?;
    let shared = Arc::new(Shared {
        dirty: std::sync::atomic::AtomicU8::new(LIST_DIRTY | QUEUES_DIRTY | SETTINGS_DIRTY),
        browser_add: Mutex::new(None),
        probe_gen: AtomicU64::new(0),
        quitting: AtomicBool::new(false),
        power: Mutex::new(None),
    });
    let view = Rc::new(RefCell::new(View {
        manager: manager.clone(),
        shown: Vec::new(),
        selected: HashSet::new(),
        anchor: None,
        rows: Rc::new(VecModel::default()),
        pending_remove: Vec::new(),
        pending_limit: None,
        pending_add: None,
        auto_name: String::new(),
        queue_ids: Vec::new(),
        glass_done: false,
        toast_gen: 0,
    }));

    {
        let ui = app.global::<Ui>();
        ui.set_rows(ModelRc::from(view.borrow().rows.clone()));
        ui.set_api_up(api_up);
        ui.set_api_text(api_text.into());
        ui.set_version(env!("CARGO_PKG_VERSION").into());
    }

    listen(&manager, shared.clone(), app.as_weak());
    wire_list(&app, &view, &shared);
    wire_add(&app, &view, &shared);
    wire_prefs(&app, &view);
    wire_queues(&app, &view);
    extras::wire(&app, &view);

    let tray = tray::Tray::new(app.as_weak());
    let has_tray = tray.is_some();

    {
        let shared = shared.clone();
        let manager = manager.clone();
        app.window().on_close_requested(move || {
            if has_tray && manager.settings().use_system_tray && !shared.quitting.load(Ordering::SeqCst) {
                slint::CloseRequestResponse::HideWindow
            } else {
                let _ = slint::quit_event_loop();
                slint::CloseRequestResponse::HideWindow
            }
        });
    }

    let timer = slint::Timer::default();
    {
        let weak = app.as_weak();
        let view = view.clone();
        let shared = shared.clone();
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(250), move || {
            let Some(app) = weak.upgrade() else { return };
            let flags = shared.dirty.swap(0, Ordering::SeqCst);
            if flags & SETTINGS_DIRTY != 0 {
                let manager = view.borrow().manager.clone();
                // Unsaved edits on the settings page stay; the site list always follows.
                if app.global::<Prefs>().get_dirty() {
                    extras::load_sites(&app, &manager.settings());
                } else {
                    load_prefs(&app, &manager);
                }
            }
            if flags & QUEUES_DIRTY != 0 {
                refresh_queues(&app, &mut view.borrow_mut());
            }
            if flags != 0 {
                refresh(&app, &mut view.borrow_mut());
            }
            let pending = shared.browser_add.lock().ok().and_then(|mut slot| slot.take());
            if let Some(req) = pending {
                show_window(&app, &view);
                open_add(&app, &view, &shared, Some(req));
            }
            let power = shared.power.lock().ok().and_then(|mut slot| slot.take());
            if let Some((queue, action)) = power {
                extras::start_power(&app, &queue, action);
            }
        });
    }

    let start_hidden = (background || manager.settings().start_minimized_to_tray) && has_tray;
    if !start_hidden {
        show_window(&app, &view);
    }
    if let Some(url) = std::env::args().skip(1).find(|a| a.starts_with("http://") || a.starts_with("https://")) {
        let req = AddRequest { url, ..Default::default() };
        show_window(&app, &view);
        open_add(&app, &view, &shared, Some(req));
    }

    slint::run_event_loop_until_quit()?;

    drop(tray);
    drop(timer);
    manager.pause_all();
    if let Ok(handle) = api {
        rt.block_on(handle.stop());
    }
    rt.shutdown_timeout(Duration::from_secs(3));
    Ok(())
}

/// A second launch asks the running copy to come forward, then exits.
async fn wake_running_copy(dir: &std::path::Path) {
    let Ok(settings) = sdm_core::Settings::load(&dir.join("settings.json")) else { return };
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("http://127.0.0.1:{}/v1/show", settings.api_port))
        .header("X-Api-Key", settings.api_key)
        .timeout(Duration::from_secs(3))
        .send()
        .await;
}

fn listen(manager: &Arc<Manager>, shared: Arc<Shared>, weak: slint::Weak<AppWindow>) {
    let mut events = manager.subscribe();
    let manager = manager.clone();
    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match events.recv().await {
                Ok(ManagerEvent::Added(_) | ManagerEvent::Changed(_) | ManagerEvent::Removed(_) | ManagerEvent::Progress { .. }) => {
                    shared.dirty.fetch_or(LIST_DIRTY, Ordering::SeqCst);
                }
                Ok(ManagerEvent::QueuesChanged) => {
                    shared.dirty.fetch_or(QUEUES_DIRTY | LIST_DIRTY, Ordering::SeqCst);
                }
                Ok(ManagerEvent::SettingsChanged) => {
                    shared.dirty.fetch_or(SETTINGS_DIRTY | LIST_DIRTY, Ordering::SeqCst);
                }
                Ok(ManagerEvent::AddRequested(req)) => {
                    if let Ok(mut slot) = shared.browser_add.lock() {
                        *slot = Some(*req);
                    }
                }
                Ok(ManagerEvent::ShowRequested) => {
                    let _ = weak.upgrade_in_event_loop(|app| app.global::<Ui>().invoke_action("show".into()));
                }
                Ok(ManagerEvent::ExitRequested) => {
                    let _ = weak.upgrade_in_event_loop(|app| app.global::<Ui>().invoke_action("quit".into()));
                }
                Ok(ManagerEvent::Done { ok, .. }) => {
                    extras::play_done(&manager.settings(), ok);
                }
                Ok(ManagerEvent::QueueFinished { name, action, .. }) => {
                    if let Ok(mut slot) = shared.power.lock() {
                        *slot = Some((name, action));
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    shared.dirty.fetch_or(LIST_DIRTY | QUEUES_DIRTY, Ordering::SeqCst);
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn show_window(app: &AppWindow, view: &Rc<RefCell<View>>) {
    show_window_with(app, view, true);
}

/// Shows the window without pulling it in front of what the user is doing.
fn show_window_quiet(app: &AppWindow, view: &Rc<RefCell<View>>) {
    show_window_with(app, view, false);
}

fn show_window_with(app: &AppWindow, view: &Rc<RefCell<View>>, forward: bool) {
    let _ = app.show();
    let weak = app.as_weak();
    let view = view.clone();
    slint::Timer::single_shot(Duration::from_millis(30), move || {
        let Some(app) = weak.upgrade() else { return };
        if let Some(hwnd) = win::hwnd_of(app.window()) {
            let mut v = view.borrow_mut();
            if !v.glass_done {
                win::apply_glass(hwnd);
                win::guard_taskbar_restart(hwnd);
                v.glass_done = true;
            }
            if forward {
                win::bring_forward(hwnd);
            }
        }
    });
}

fn toast(app: &AppWindow, view: &Rc<RefCell<View>>, text: impl Into<SharedString>, tone: i32) {
    let ui = app.global::<Ui>();
    ui.set_toast(text.into());
    ui.set_toast_tone(tone);
    let generation = {
        let mut v = view.borrow_mut();
        v.toast_gen += 1;
        v.toast_gen
    };
    let weak = app.as_weak();
    let view = view.clone();
    slint::Timer::single_shot(Duration::from_millis(3200), move || {
        if view.borrow().toast_gen == generation
            && let Some(app) = weak.upgrade()
        {
            app.global::<Ui>().set_toast(SharedString::new());
        }
    });
}

// ---------- the list ----------

fn glyph(code: u32) -> SharedString {
    char::from_u32(code).map(|c| c.to_string()).unwrap_or_default().into()
}

fn category_icon(icon: &str) -> u32 {
    match icon {
        "zip" => 0xf1c6,
        "application" => 0xf2d0,
        "video" => 0xf1c8,
        "music" => 0xf1c7,
        "picture" => 0xf1c5,
        "document" => 0xf0f6,
        _ => 0xf016,
    }
}

fn is_decimal(mode: UnitMode) -> bool {
    matches!(mode, UnitMode::DecimalBytes | UnitMode::DecimalBits)
}

fn percent(d: &DownloadView) -> Option<u64> {
    d.size.filter(|s| *s > 0).map(|s| (d.downloaded.min(s) * 100) / s)
}

fn make_row(d: &DownloadView, selected: bool, categories: &[Category], decimal: bool) -> Row {
    let pct = percent(d);
    let (status, tone) = match d.status {
        Status::Completed => ("Done".to_string(), 1),
        Status::Downloading => (pct.map(|p| format!("Downloading {p}%")).unwrap_or_else(|| "Downloading".into()), 2),
        Status::Retrying => ("Trying again".to_string(), 5),
        Status::Paused => (pct.map(|p| format!("Paused at {p}%")).unwrap_or_else(|| "Paused".into()), 3),
        Status::Queued => ("Waiting in queue".to_string(), 5),
        Status::Failed => ("Failed".to_string(), 4),
        Status::Added => ("Not started".to_string(), 0),
    };
    let running = d.status.is_running();
    let finished = d.status == Status::Completed;
    let icon = d
        .category_id
        .and_then(|cid| categories.iter().find(|c| c.id == cid))
        .map(|c| category_icon(&c.icon))
        .unwrap_or(0xf016);
    let size = match (d.size, finished) {
        (Some(total), _) => fmt::size(total, decimal),
        (None, true) => fmt::size(d.downloaded, decimal),
        (None, false) if d.downloaded > 0 => fmt::size(d.downloaded, decimal),
        _ => String::new(),
    };
    Row {
        id: d.id as i32,
        name: d.name.as_str().into(),
        icon: glyph(icon),
        size: size.into(),
        status: status.into(),
        tone,
        progress: d.size.filter(|s| *s > 0).map(|s| d.downloaded as f32 / s as f32).unwrap_or(0.0).clamp(0.0, 1.0),
        show_progress: !finished && (running || d.downloaded > 0),
        speed: if running && d.speed > 0 { fmt::speed(d.speed, decimal).into() } else { SharedString::new() },
        eta: if running { d.eta_secs.map(fmt::eta).unwrap_or_default().into() } else { SharedString::new() },
        added: fmt::when(d.date_added).into(),
        selected,
        running,
        finished,
        error: d.error.clone().unwrap_or_default().into(),
    }
}

fn refresh(app: &AppWindow, v: &mut View) {
    let ui = app.global::<Ui>();
    let settings = v.manager.settings();
    let decimal = is_decimal(settings.size_unit);
    let speed_decimal = is_decimal(settings.speed_unit);
    let all = v.manager.list();
    let categories = v.manager.categories();
    let queues = v.manager.queues();
    let active: HashSet<i64> = v.manager.active_queue_ids().into_iter().collect();
    let filter = ui.get_filter().to_string();
    let search = ui.get_search().to_lowercase();

    let mut queue_order: Option<Vec<i64>> = None;
    let mut shown: Vec<DownloadView> = all
        .iter()
        .filter(|d| match filter.as_str() {
            "all" => true,
            "running" => d.status.is_running() || d.status == Status::Queued,
            "done" => d.status == Status::Completed,
            "unfinished" => d.status != Status::Completed,
            f => {
                if let Some(id) = f.strip_prefix("cat:").and_then(|n| n.parse::<i64>().ok()) {
                    d.category_id == Some(id)
                } else if let Some(id) = f.strip_prefix("queue:").and_then(|n| n.parse::<i64>().ok()) {
                    d.queue_id == Some(id)
                } else {
                    true
                }
            }
        })
        .filter(|d| search.is_empty() || d.name.to_lowercase().contains(&search) || d.url.to_lowercase().contains(&search))
        .cloned()
        .collect();
    if let Some(qid) = filter.strip_prefix("queue:").and_then(|n| n.parse::<i64>().ok())
        && let Some(q) = queues.iter().find(|q| q.id == qid)
    {
        queue_order = Some(q.items.clone());
    }
    match &queue_order {
        Some(order) => shown.sort_by_key(|d| order.iter().position(|i| *i == d.id).unwrap_or(usize::MAX)),
        None => shown.sort_by(|a, b| b.date_added.cmp(&a.date_added).then(b.id.cmp(&a.id))),
    }

    let present: HashSet<i64> = shown.iter().map(|d| d.id).collect();
    v.selected.retain(|id| present.contains(id));

    let rows: Vec<Row> = shown.iter().map(|d| make_row(d, v.selected.contains(&d.id), &categories, decimal)).collect();
    let same_ids = v.rows.row_count() == rows.len() && v.shown.iter().zip(&shown).all(|(a, b)| a.id == b.id);
    if same_ids {
        for (i, row) in rows.into_iter().enumerate() {
            if v.rows.row_data(i).as_ref() != Some(&row) {
                v.rows.set_row_data(i, row);
            }
        }
    } else {
        v.rows.set_vec(rows);
    }
    v.shown = shown;

    let sel: Vec<&DownloadView> = v.shown.iter().filter(|d| v.selected.contains(&d.id)).collect();
    ui.set_selected_count(sel.len() as i32);
    ui.set_any_running_selected(sel.iter().any(|d| d.status.is_running() || d.status == Status::Queued));
    ui.set_any_paused_selected(sel.iter().any(|d| !d.status.is_running() && d.status != Status::Completed));
    ui.set_selected_finished(!sel.is_empty() && sel.iter().all(|d| d.status == Status::Completed));

    let count = |f: &dyn Fn(&DownloadView) -> bool| all.iter().filter(|&d| f(d)).count();
    let running_now = count(&|d| d.status.is_running());
    let waiting = count(&|d| d.status == Status::Queued);
    let nav = |key: &str, label: &str, icon: u32, n: usize, busy: bool| NavEntry {
        key: key.into(),
        label: label.into(),
        icon: glyph(icon),
        count: if n > 0 { n.to_string().into() } else { SharedString::new() },
        busy,
    };
    ui.set_views(ModelRc::new(VecModel::from(vec![
        nav("all", "All downloads", 0xf03a, all.len(), false),
        nav("running", "Downloading", 0xf019, running_now + waiting, running_now > 0),
        nav("done", "Finished", 0xf058, count(&|d| d.status == Status::Completed), false),
        nav("unfinished", "Unfinished", 0xf017, count(&|d| d.status != Status::Completed), false),
    ])));
    ui.set_types(ModelRc::new(VecModel::from(
        categories
            .iter()
            .map(|c| nav(&format!("cat:{}", c.id), &c.name, category_icon(&c.icon), count(&|d| d.category_id == Some(c.id)), false))
            .collect::<Vec<_>>(),
    )));
    ui.set_queues_nav(ModelRc::new(VecModel::from(
        queues
            .iter()
            .map(|q| nav(&format!("queue:{}", q.id), &q.name, 0xf0ae, q.items.len(), active.contains(&q.id)))
            .collect::<Vec<_>>(),
    )));
    let filter_queue = filter.strip_prefix("queue:").and_then(|n| n.parse::<i64>().ok());
    ui.set_filter_is_queue(filter_queue.is_some());
    ui.set_filter_queue_running(filter_queue.is_some_and(|q| active.contains(&q)));

    let total_speed: u64 = all.iter().filter(|d| d.status.is_running()).map(|d| d.speed).sum();
    ui.set_total_speed(if total_speed > 0 { fmt::speed(total_speed, speed_decimal).into() } else { SharedString::new() });
    ui.set_running_text(
        match (running_now, waiting) {
            (0, 0) => "Nothing downloading".to_string(),
            (r, 0) => format!("{r} downloading"),
            (0, w) => format!("{w} waiting"),
            (r, w) => format!("{r} downloading, {w} waiting"),
        }
        .into(),
    );
    ui.set_tint(settings.window_tint.clamp(0.2, 0.9));

    let (title, hint) = if all.is_empty() {
        ("No downloads yet", "Copy a link and press Ctrl+V, or click Add link. Downloads from your browser land here too.")
    } else if !search.is_empty() {
        ("Nothing matches that search", "Try fewer words, or clear the search box.")
    } else {
        ("Nothing here right now", "Downloads show up here when they fit this view.")
    };
    ui.set_empty_title(title.into());
    ui.set_empty_hint(hint.into());

    let names: Vec<SharedString> = queues.iter().map(|q| SharedString::from(q.name.as_str())).collect();
    v.queue_ids = queues.iter().map(|q| q.id).collect();
    ui.set_queue_names(ModelRc::new(VecModel::from(names)));
}

fn refresh_queues(app: &AppWindow, v: &mut View) {
    let active: HashSet<i64> = v.manager.active_queue_ids().into_iter().collect();
    let all = v.manager.list();
    let list: Vec<QueueView> = v
        .manager
        .queues()
        .iter()
        .map(|q| QueueView {
            id: q.id as i32,
            name: q.name.as_str().into(),
            max: q.max_concurrent.to_string().into(),
            running: active.contains(&q.id),
            count: q.items.iter().filter(|id| all.iter().any(|d| d.id == **id && d.status != Status::Completed)).count() as i32,
            days: ModelRc::new(VecModel::from(Weekday::ALL.iter().map(|d| q.schedule.days.contains(d)).collect::<Vec<_>>())),
            start_on: q.schedule.enabled_start,
            start: q.schedule.start.clone().unwrap_or_default().into(),
            end_on: q.schedule.enabled_end,
            end: q.schedule.end.clone().unwrap_or_default().into(),
            stop_empty: q.stop_on_empty,
            removable: q.id != 0,
            power: q.power_action.index() as i32,
        })
        .collect();
    app.global::<Ui>().set_queues(ModelRc::new(VecModel::from(list)));
}

fn selected_ids(v: &View) -> Vec<i64> {
    v.shown.iter().filter(|d| v.selected.contains(&d.id)).map(|d| d.id).collect()
}

fn wire_list(app: &AppWindow, view: &Rc<RefCell<View>>, shared: &Arc<Shared>) {
    let ui = app.global::<Ui>();
    let weak = app.as_weak();

    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_set_filter(move |key| {
            let Some(app) = weak.upgrade() else { return };
            let ui = app.global::<Ui>();
            ui.set_filter(key);
            ui.set_page("list".into());
            let mut v = view.borrow_mut();
            v.selected.clear();
            v.anchor = None;
            refresh(&app, &mut v);
            app.invoke_focus_list();
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_set_page(move |page| {
            let Some(app) = weak.upgrade() else { return };
            if page == "settings" {
                load_prefs(&app, &view.borrow().manager);
            }
            if page == "queues" {
                refresh_queues(&app, &mut view.borrow_mut());
            }
            if page == "about" {
                extras::check_update_once(&app);
            }
            app.global::<Ui>().set_page(page);
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_search_edited(move |_| {
            if let Some(app) = weak.upgrade() {
                refresh(&app, &mut view.borrow_mut());
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_row_clicked(move |index, ctrl, shift| {
            let Some(app) = weak.upgrade() else { return };
            let mut v = view.borrow_mut();
            let index = index as usize;
            let Some(id) = v.shown.get(index).map(|d| d.id) else { return };
            if shift {
                let from = v.anchor.unwrap_or(index);
                let (a, b) = if from <= index { (from, index) } else { (index, from) };
                if !ctrl {
                    v.selected.clear();
                }
                let ids: Vec<i64> = v.shown[a..=b].iter().map(|d| d.id).collect();
                v.selected.extend(ids);
            } else if ctrl {
                if !v.selected.remove(&id) {
                    v.selected.insert(id);
                }
                v.anchor = Some(index);
            } else {
                v.selected.clear();
                v.selected.insert(id);
                v.anchor = Some(index);
            }
            refresh(&app, &mut v);
            app.invoke_focus_list();
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_select_all(move || {
            let Some(app) = weak.upgrade() else { return };
            let mut v = view.borrow_mut();
            let ids: Vec<i64> = v.shown.iter().map(|d| d.id).collect();
            v.selected = ids.into_iter().collect();
            refresh(&app, &mut v);
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_row_double_clicked(move |index| {
            let Some(app) = weak.upgrade() else { return };
            let (manager, item) = {
                let v = view.borrow();
                (v.manager.clone(), v.shown.get(index as usize).cloned())
            };
            let Some(item) = item else { return };
            match item.status {
                Status::Completed => open_file(&app, &view, &item),
                s if s.is_running() => {}
                _ => manager.resume(item.id),
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_move_to_queue(move |index| {
            let Some(app) = weak.upgrade() else { return };
            let (manager, ids, qid) = {
                let v = view.borrow();
                (v.manager.clone(), selected_ids(&v), v.queue_ids.get(index as usize).copied())
            };
            let Some(qid) = qid else { return };
            match manager.move_to_queue(&ids, qid) {
                Ok(()) => toast(&app, &view, "Moved to the queue", 1),
                Err(e) => toast(&app, &view, format!("Could not move them: {e}"), 4),
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_confirm_remove(move |delete_files| {
            let Some(app) = weak.upgrade() else { return };
            app.global::<Confirm>().set_open(false);
            let (manager, ids) = {
                let mut v = view.borrow_mut();
                (v.manager.clone(), std::mem::take(&mut v.pending_remove))
            };
            let mut failed = 0;
            for id in &ids {
                if manager.remove(*id, delete_files).is_err() {
                    failed += 1;
                }
            }
            view.borrow_mut().selected.clear();
            if failed > 0 {
                toast(&app, &view, format!("{failed} could not be removed"), 4);
            } else {
                toast(&app, &view, if ids.len() == 1 { "Removed".to_string() } else { format!("Removed {}", ids.len()) }, 0);
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_row_menu(move |index| {
            let Some(app) = weak.upgrade() else { return };
            let mut v = view.borrow_mut();
            let Some(id) = v.shown.get(index as usize).map(|d| d.id) else { return };
            if !v.selected.contains(&id) {
                v.selected.clear();
                v.selected.insert(id);
                v.anchor = Some(index as usize);
                refresh(&app, &mut v);
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        app.global::<Limit>().on_save(move || {
            let Some(app) = weak.upgrade() else { return };
            let limit = app.global::<Limit>();
            let Some(id) = view.borrow().pending_limit else {
                limit.set_open(false);
                return;
            };
            let rate = if limit.get_on() {
                match parse_num(&limit.get_kb(), "The speed limit", 1, 10_000_000) {
                    Ok(kb) => kb * 1024,
                    Err(e) => {
                        limit.set_error(e.into());
                        return;
                    }
                }
            } else {
                0
            };
            let manager = view.borrow().manager.clone();
            manager.set_speed_limit(id, rate);
            view.borrow_mut().pending_limit = None;
            limit.set_open(false);
            let text = if rate == 0 { "Speed limit removed".to_string() } else { format!("Limited to {} KB per second", rate / 1024) };
            toast(&app, &view, text, 1);
        });
    }
    ui.on_open_url(|url| win::shell_open(&url));
    {
        let (weak, view, shared) = (weak.clone(), view.clone(), shared.clone());
        ui.on_action(move |name| {
            let Some(app) = weak.upgrade() else { return };
            run_action(&app, &view, &shared, name.as_str());
        });
    }
}

fn open_file(app: &AppWindow, view: &Rc<RefCell<View>>, item: &DownloadView) {
    let path = PathBuf::from(&item.folder).join(&item.name);
    if path.exists() {
        win::shell_open(&path.display().to_string());
    } else {
        toast(app, view, "That file is no longer on the disk", 4);
    }
}

fn run_action(app: &AppWindow, view: &Rc<RefCell<View>>, shared: &Arc<Shared>, name: &str) {
    let (manager, ids, items) = {
        let v = view.borrow();
        let ids = selected_ids(&v);
        let items: Vec<DownloadView> = v.shown.iter().filter(|d| v.selected.contains(&d.id)).cloned().collect();
        (v.manager.clone(), ids, items)
    };
    match name {
        "show" => show_window(app, view),
        "quit" => {
            shared.quitting.store(true, Ordering::SeqCst);
            let _ = slint::quit_event_loop();
        }
        "add" | "paste" => {
            show_window(app, view);
            open_add(app, view, shared, None);
        }
        "batch" => {
            show_window(app, view);
            extras::open_batch(app);
        }
        "check-file" => {
            if let [d] = items.as_slice()
                && d.status == Status::Completed
            {
                let path = std::path::Path::new(&d.folder).join(&d.name);
                if path.is_file() {
                    extras::open_sum(app, path);
                } else {
                    toast(app, view, "The file is not there any more", 4);
                }
            }
        }
        "resume" => {
            for d in items.iter().filter(|d| !d.status.is_running() && d.status != Status::Completed) {
                manager.resume(d.id);
            }
        }
        "pause" => {
            for d in items.iter().filter(|d| d.status.is_running() || d.status == Status::Queued) {
                manager.pause(d.id);
            }
        }
        "toggle" => {
            for d in &items {
                if d.status.is_running() || d.status == Status::Queued {
                    manager.pause(d.id);
                } else if d.status != Status::Completed {
                    manager.resume(d.id);
                }
            }
        }
        "pause-all" => manager.pause_all(),
        "resume-all" => manager.resume_all(),
        "toggle-queue" => {
            let filter = app.global::<Ui>().get_filter().to_string();
            if let Some(qid) = filter.strip_prefix("queue:").and_then(|n| n.parse::<i64>().ok()) {
                if manager.active_queue_ids().contains(&qid) {
                    manager.stop_queue(qid);
                } else {
                    manager.start_queue(qid);
                }
                shared.dirty.fetch_or(LIST_DIRTY | QUEUES_DIRTY, Ordering::SeqCst);
            }
        }
        "remove" => {
            if ids.is_empty() {
                return;
            }
            let confirm = app.global::<Confirm>();
            let title = if ids.len() == 1 { format!("Remove {}?", items[0].name) } else { format!("Remove {} downloads?", ids.len()) };
            confirm.set_title(title.into());
            confirm.set_body("They leave the list. Files already on the disk stay there unless you turn on the switch below.".into());
            confirm.set_delete_files(false);
            confirm.set_open(true);
            view.borrow_mut().pending_remove = ids;
        }
        "open" => {
            if let Some(item) = items.first() {
                if item.status == Status::Completed {
                    open_file(app, view, item);
                } else if !item.status.is_running() {
                    manager.resume(item.id);
                }
            }
        }
        "folder" => {
            if let Some(item) = items.first() {
                win::show_in_folder(&PathBuf::from(&item.folder).join(&item.name));
            }
        }
        "copy-link" => {
            let links: Vec<&str> = items.iter().map(|d| d.url.as_str()).collect();
            if !links.is_empty() && set_clipboard(&links.join("\r\n")) {
                toast(app, view, if links.len() == 1 { "Link copied".to_string() } else { format!("{} links copied", links.len()) }, 1);
            }
        }
        "speed-limit" => {
            let [item] = items.as_slice() else { return };
            let limit = app.global::<Limit>();
            limit.set_title(item.name.clone().into());
            limit.set_on(item.speed_limit > 0);
            let kb = if item.speed_limit > 0 { (item.speed_limit / 1024).max(1) } else { 512 };
            limit.set_kb(kb.to_string().into());
            limit.set_error("".into());
            limit.set_open(true);
            view.borrow_mut().pending_limit = Some(item.id);
            return;
        }
        "queue-up" | "queue-down" => {
            let filter = app.global::<Ui>().get_filter().to_string();
            let Some(qid) = filter.strip_prefix("queue:").and_then(|n| n.parse::<i64>().ok()) else { return };
            let Some(mut queue) = manager.queues().into_iter().find(|q| q.id == qid) else { return };
            move_in_queue(&mut queue, &ids, name == "queue-up");
            let _ = manager.save_queue(queue);
        }
        _ => {}
    }
    shared.dirty.fetch_or(LIST_DIRTY, Ordering::SeqCst);
}

fn move_in_queue(queue: &mut Queue, ids: &[i64], up: bool) {
    let picked: HashSet<i64> = ids.iter().copied().collect();
    let items = &mut queue.items;
    if up {
        for i in 1..items.len() {
            if picked.contains(&items[i]) && !picked.contains(&items[i - 1]) {
                items.swap(i, i - 1);
            }
        }
    } else {
        for i in (0..items.len().saturating_sub(1)).rev() {
            if picked.contains(&items[i]) && !picked.contains(&items[i + 1]) {
                items.swap(i, i + 1);
            }
        }
    }
}

fn get_clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn set_clipboard(text: &str) -> bool {
    arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())).is_ok()
}

// ---------- the add box ----------

fn looks_like_link(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("http://") || t.starts_with("https://")) && !t.contains(char::is_whitespace)
}

fn open_add(app: &AppWindow, view: &Rc<RefCell<View>>, shared: &Arc<Shared>, req: Option<AddRequest>) {
    let add = app.global::<AddBox>();
    let manager = view.borrow().manager.clone();
    let settings = manager.settings();
    let url = match &req {
        Some(r) => r.url.clone(),
        None => get_clipboard().filter(|c| looks_like_link(c)).map(|c| c.trim().to_string()).unwrap_or_default(),
    };
    let name = req.as_ref().and_then(|r| r.name.clone().or(r.suggested_name.clone())).unwrap_or_default();
    let folder = req.as_ref().and_then(|r| r.folder.clone()).unwrap_or_else(|| settings.default_download_folder.clone());
    {
        let mut v = view.borrow_mut();
        v.pending_add = req;
        v.auto_name = name.clone();
    }
    add.set_url(url.clone().into());
    add.set_name(name.into());
    add.set_folder(folder.into());
    add.set_queue_index(0);
    add.set_info(SharedString::new());
    add.set_info_tone(0);
    add.set_can_add(looks_like_link(&url));
    add.set_auth_on(false);
    add.set_user(SharedString::new());
    add.set_pass(SharedString::new());
    add.set_open(true);
    app.invoke_focus_add();
    if looks_like_link(&url) {
        start_probe(app, view, shared, url, Duration::ZERO);
    }
}

fn start_probe(app: &AppWindow, view: &Rc<RefCell<View>>, shared: &Arc<Shared>, url: String, wait: Duration) {
    let generation = shared.probe_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let (manager, headers, decimal) = {
        let v = view.borrow();
        let headers = v.pending_add.as_ref().filter(|r| r.url == url).map(|r| r.headers.clone()).unwrap_or_default();
        (v.manager.clone(), headers, is_decimal(v.manager.settings().size_unit))
    };
    app.global::<AddBox>().set_checking(true);
    let weak = app.as_weak();
    let shared = shared.clone();
    tokio::spawn(async move {
        tokio::time::sleep(wait).await;
        if shared.probe_gen.load(Ordering::SeqCst) != generation {
            return;
        }
        let result = manager.probe(&url, headers).await;
        if shared.probe_gen.load(Ordering::SeqCst) != generation {
            return;
        }
        let (info, tone, name) = match result {
            Ok(p) if p.is_web_page => ("This link opens a web page, not a file. You can still add it.".to_string(), 4, Some(p.file_name)),
            Ok(p) if p.needs_auth => ("The site wants you signed in. Try the download from your browser instead.".to_string(), 4, None),
            Ok(p) => {
                let size = p.total.map(|t| fmt::size(t, decimal)).unwrap_or_else(|| "Size not known".into());
                let resume = if p.resumable { "can pause and resume" } else { "cannot resume if it stops" };
                (format!("{size}, {resume}."), 1, Some(p.file_name))
            }
            Err(e) => (format!("Could not reach that link: {e}"), 4, None),
        };
        let _ = weak.upgrade_in_event_loop(move |app| {
            let add = app.global::<AddBox>();
            add.set_checking(false);
            add.set_info(info.into());
            add.set_info_tone(tone);
            if let Some(name) = name.filter(|n| !n.is_empty())
                && add.get_name().is_empty()
            {
                add.set_name(name.into());
            }
        });
    });
}

fn wire_add(app: &AppWindow, view: &Rc<RefCell<View>>, shared: &Arc<Shared>) {
    let add = app.global::<AddBox>();
    let weak = app.as_weak();
    {
        let (weak, view, shared) = (weak.clone(), view.clone(), shared.clone());
        add.on_url_edited(move |text| {
            let Some(app) = weak.upgrade() else { return };
            let add = app.global::<AddBox>();
            let ok = looks_like_link(&text);
            add.set_can_add(ok);
            add.set_info(SharedString::new());
            if ok {
                start_probe(&app, &view, &shared, text.trim().to_string(), Duration::from_millis(450));
            } else {
                shared.probe_gen.fetch_add(1, Ordering::SeqCst);
                add.set_checking(false);
            }
        });
    }
    {
        let weak = weak.clone();
        add.on_browse(move || {
            let Some(app) = weak.upgrade() else { return };
            let add = app.global::<AddBox>();
            if let Some(dir) = pick_folder(&add.get_folder()) {
                add.set_folder(dir.into());
            }
        });
    }
    {
        let (weak, shared) = (weak.clone(), shared.clone());
        add.on_cancel(move || {
            shared.probe_gen.fetch_add(1, Ordering::SeqCst);
            if let Some(app) = weak.upgrade() {
                app.global::<AddBox>().set_open(false);
                app.invoke_focus_list();
            }
        });
    }
    {
        let (weak, view, shared) = (weak.clone(), view.clone(), shared.clone());
        add.on_add(move |start_now| {
            let Some(app) = weak.upgrade() else { return };
            let add = app.global::<AddBox>();
            let url = add.get_url().trim().to_string();
            let (manager, base, queue_id) = {
                let mut v = view.borrow_mut();
                let base = v.pending_add.take().filter(|r| r.url == url).unwrap_or_default();
                (v.manager.clone(), base, v.queue_ids.get(add.get_queue_index().max(0) as usize).copied())
            };
            let name = add.get_name().trim().to_string();
            let user = add.get_user().trim().to_string();
            let basic_auth = if add.get_auth_on() && !user.is_empty() { Some((user, add.get_pass().to_string())) } else { base.basic_auth.clone() };
            let req = AddRequest {
                basic_auth,
                url,
                name: if name.is_empty() { None } else { Some(name.clone()) },
                folder: Some(add.get_folder().to_string()).filter(|f| !f.trim().is_empty()),
                queue_id: if start_now { None } else { queue_id },
                start: start_now,
                start_queue: false,
                ..base
            };
            match manager.add(req) {
                Ok(_) => {
                    shared.probe_gen.fetch_add(1, Ordering::SeqCst);
                    add.set_open(false);
                    toast(&app, &view, if start_now { "Downloading" } else { "Added to the queue" }, 1);
                    shared.dirty.fetch_or(LIST_DIRTY, Ordering::SeqCst);
                    app.invoke_focus_list();
                }
                Err(e) => {
                    add.set_info(format!("Could not add it: {e}").into());
                    add.set_info_tone(4);
                }
            }
        });
    }
}

fn pick_folder(start: &str) -> Option<String> {
    let mut dialog = rfd::FileDialog::new().set_title("Choose a folder");
    if !start.is_empty() && std::path::Path::new(start).is_dir() {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder().map(|p| p.display().to_string())
}

// ---------- settings ----------

fn load_prefs(app: &AppWindow, manager: &Arc<Manager>) {
    let s = manager.settings();
    let p = app.global::<Prefs>();
    p.set_folder(s.default_download_folder.as_str().into());
    p.set_threads(s.thread_count.to_string().into());
    p.set_max_at_once(s.max_concurrent_downloads.to_string().into());
    p.set_retries(s.max_retry_count.to_string().into());
    p.set_limit_on(s.speed_limit_mode != SpeedLimitMode::Disabled);
    p.set_limit_kb((s.speed_limit / 1024).to_string().into());
    p.set_use_temp(s.append_extension_to_incomplete_downloads);
    p.set_sparse(s.use_sparse_file_allocation);
    p.set_server_time(s.use_server_last_modified_time);
    p.set_delete_partial(s.delete_partial_file_on_cancellation);
    p.set_use_categories(s.use_category_by_default);
    p.set_dynamic_parts(s.dynamic_part_creation);
    p.set_decimal_units(is_decimal(s.size_unit));
    p.set_port(s.api_port.to_string().into());
    p.set_autostart(s.auto_start_on_boot);
    p.set_tray(s.use_system_tray);
    p.set_start_hidden(s.start_minimized_to_tray);
    p.set_tint(s.window_tint);
    extras::load_prefs(app, &s);
    p.set_dirty(false);
}

fn parse_num(text: &str, what: &str, min: u64, max: u64) -> Result<u64, String> {
    match text.trim().parse::<u64>() {
        Ok(n) if (min..=max).contains(&n) => Ok(n),
        _ => Err(format!("{what} must be a number from {min} to {max}")),
    }
}

fn save_prefs(app: &AppWindow, manager: &Arc<Manager>) -> Result<(), String> {
    let p = app.global::<Prefs>();
    let mut s = manager.settings();
    let was_autostart = s.auto_start_on_boot;
    s.default_download_folder = p.get_folder().to_string();
    s.thread_count = parse_num(&p.get_threads(), "Connections", 1, 32)? as u32;
    s.max_concurrent_downloads = parse_num(&p.get_max_at_once(), "Downloads at the same time", 0, 50)? as u32;
    s.max_retry_count = parse_num(&p.get_retries(), "Tries", 0, 50)? as u32;
    s.speed_limit_mode = if p.get_limit_on() { SpeedLimitMode::Enabled } else { SpeedLimitMode::Disabled };
    if p.get_limit_on() || !p.get_limit_kb().trim().is_empty() {
        s.speed_limit = parse_num(&p.get_limit_kb(), "The speed limit", 1, 10_000_000)? * 1024;
    }
    s.append_extension_to_incomplete_downloads = p.get_use_temp();
    s.use_sparse_file_allocation = p.get_sparse();
    s.use_server_last_modified_time = p.get_server_time();
    s.delete_partial_file_on_cancellation = p.get_delete_partial();
    s.use_category_by_default = p.get_use_categories();
    s.dynamic_part_creation = p.get_dynamic_parts();
    let unit = if p.get_decimal_units() { UnitMode::DecimalBytes } else { UnitMode::BinaryBytes };
    s.size_unit = unit;
    s.speed_unit = unit;
    s.api_port = parse_num(&p.get_port(), "The port", 1024, 65535)? as u16;
    s.auto_start_on_boot = p.get_autostart();
    s.use_system_tray = p.get_tray();
    s.start_minimized_to_tray = p.get_start_hidden();
    s.window_tint = p.get_tint().clamp(0.2, 0.9);
    extras::save_prefs(app, &mut s)?;
    let autostart = s.auto_start_on_boot;
    manager.update_settings(s).map_err(|e| format!("Could not save: {e}"))?;
    if autostart != was_autostart {
        win::set_autostart(autostart).map_err(|e| format!("Saved, but starting with Windows could not be set: {e}"))?;
    }
    Ok(())
}

fn wire_prefs(app: &AppWindow, view: &Rc<RefCell<View>>) {
    let prefs = app.global::<Prefs>();
    let weak = app.as_weak();
    {
        let weak = weak.clone();
        prefs.on_changed(move || {
            if let Some(app) = weak.upgrade() {
                app.global::<Prefs>().set_dirty(true);
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        prefs.on_save(move || {
            let Some(app) = weak.upgrade() else { return };
            let manager = view.borrow().manager.clone();
            match save_prefs(&app, &manager) {
                Ok(()) => {
                    app.global::<Prefs>().set_dirty(false);
                    toast(&app, &view, "Settings saved", 1);
                }
                Err(e) => toast(&app, &view, e, 4),
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        prefs.on_reset(move || {
            if let Some(app) = weak.upgrade() {
                load_prefs(&app, &view.borrow().manager);
            }
        });
    }
    {
        let weak = weak.clone();
        prefs.on_browse_folder(move || {
            let Some(app) = weak.upgrade() else { return };
            let p = app.global::<Prefs>();
            if let Some(dir) = pick_folder(&p.get_folder()) {
                p.set_folder(dir.into());
                p.set_dirty(true);
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        prefs.on_copy_key(move || {
            let Some(app) = weak.upgrade() else { return };
            let key = view.borrow().manager.settings().api_key;
            if set_clipboard(&key) {
                toast(&app, &view, "Browser key copied", 1);
            }
        });
    }
}

// ---------- queues ----------

fn wire_queues(app: &AppWindow, view: &Rc<RefCell<View>>) {
    let ui = app.global::<Ui>();
    let weak = app.as_weak();
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_queue_save(move |id, name, max, days, start_on, start, end_on, end, stop_empty, power| {
            let Some(app) = weak.upgrade() else { return };
            let manager = view.borrow().manager.clone();
            let Some(mut queue) = manager.queues().into_iter().find(|q| q.id == id as i64) else { return };
            let Ok(max) = max.trim().parse::<u32>() else {
                toast(&app, &view, "Downloads at the same time must be a number", 4);
                return;
            };
            let valid_time = |t: &str| chrono::NaiveTime::parse_from_str(t.trim(), "%H:%M").is_ok();
            if (start_on && !valid_time(&start)) || (end_on && !valid_time(&end)) {
                toast(&app, &view, "Times look like 02:30, on a 24 hour clock", 4);
                return;
            }
            queue.name = name.trim().to_string();
            queue.max_concurrent = max.clamp(1, 50);
            queue.schedule.days = Weekday::ALL.iter().enumerate().filter(|(i, _)| days.row_data(*i).unwrap_or(false)).map(|(_, d)| *d).collect();
            queue.schedule.enabled_start = start_on;
            queue.schedule.start = Some(start.trim().to_string()).filter(|s| !s.is_empty());
            queue.schedule.enabled_end = end_on;
            queue.schedule.end = Some(end.trim().to_string()).filter(|s| !s.is_empty());
            queue.stop_on_empty = stop_empty;
            queue.power_action = PowerAction::ALL[power.clamp(0, 4) as usize];
            match manager.save_queue(queue) {
                Ok(_) => toast(&app, &view, "Queue saved", 1),
                Err(e) => toast(&app, &view, format!("Could not save: {e}"), 4),
            }
        });
    }
    {
        let view = view.clone();
        ui.on_queue_toggle(move |id| {
            let manager = view.borrow().manager.clone();
            let id = id as i64;
            if manager.active_queue_ids().contains(&id) {
                manager.stop_queue(id);
            } else {
                manager.start_queue(id);
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_queue_delete(move |id| {
            let Some(app) = weak.upgrade() else { return };
            let manager = view.borrow().manager.clone();
            match manager.delete_queue(id as i64) {
                Ok(()) => toast(&app, &view, "Queue deleted. Its downloads stay in the list.", 0),
                Err(e) => toast(&app, &view, format!("Could not delete: {e}"), 4),
            }
        });
    }
    {
        let (weak, view) = (weak.clone(), view.clone());
        ui.on_queue_new(move || {
            let Some(app) = weak.upgrade() else { return };
            let manager = view.borrow().manager.clone();
            let n = manager.queues().len();
            let queue = Queue { id: -1, name: format!("Queue {n}"), ..Queue::main() };
            if let Err(e) = manager.save_queue(queue) {
                toast(&app, &view, format!("Could not add a queue: {e}"), 4);
            }
        });
    }
}
