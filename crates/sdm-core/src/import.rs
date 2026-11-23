use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use sdm_engine::Part;
use serde::Deserialize;
use serde_json::Value;

use crate::model::{Category, DownloadItem, Queue, ResumeState, Schedule, SourceKind, Status, Weekday, default_categories};
use crate::settings::{Settings, SpeedLimitMode, UnitMode};
use crate::store::Store;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub downloads: usize,
    pub queues: usize,
    pub categories: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

#[derive(Default)]
pub struct AbdmData {
    pub settings: Option<serde_json::Map<String, Value>>,
    pub items: Vec<DownloadItem>,
    pub queues: Vec<Queue>,
    pub categories: Vec<Category>,
    pub errors: Vec<String>,
}

pub fn default_abdm_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join(".abdm").join("config"))
}

/// Accepts the `.abdm` folder, its `config` folder, or a portable data folder.
pub fn locate(path: &Path) -> Option<PathBuf> {
    [path.join("config"), path.to_path_buf()]
        .into_iter()
        .find(|dir| dir.join("appSettings.json").is_file() || dir.join("download_db").is_dir())
}

fn read_json(path: &Path) -> std::result::Result<Value, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let text = fs::read_to_string(path).map_err(|e| format!("{name}: could not be read ({:?})", e.kind()))?;
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Err(format!("{name}: file is empty"));
    }
    serde_json::from_str(text).map_err(|e| format!("{name}: not valid JSON ({:?}, line {})", e.classify(), e.line()))
}

fn parse<T: for<'de> Deserialize<'de>>(value: Value, name: &str) -> std::result::Result<T, String> {
    serde_json::from_value(value).map_err(|e| format!("{name}: unexpected shape ({:?})", e.classify()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbdmItem {
    #[serde(rename = "type")]
    kind: Option<String>,
    link: String,
    headers: Option<BTreeMap<String, String>>,
    username: Option<String>,
    password: Option<String>,
    download_page: Option<String>,
    user_agent: Option<String>,
    #[serde(rename = "serverETag")]
    server_etag: Option<String>,
    id: i64,
    folder: String,
    name: String,
    content_length: Option<i64>,
    date_added: Option<i64>,
    start_time: Option<i64>,
    complete_time: Option<i64>,
    status: Option<String>,
    preferred_connection_count: Option<i64>,
    speed_limit: Option<i64>,
    file_checksum: Option<String>,
}

#[derive(Deserialize)]
struct AbdmParts {
    #[serde(default)]
    list: Vec<Part>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbdmQueue {
    id: i64,
    name: String,
    #[serde(default)]
    max_concurrent: Option<u32>,
    #[serde(default)]
    queue_items: Vec<i64>,
    #[serde(default)]
    scheduled_times: Option<AbdmSchedule>,
    #[serde(default)]
    stop_queue_on_empty: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbdmSchedule {
    #[serde(default)]
    days_of_week: Vec<Weekday>,
    start_time: Option<String>,
    end_time: Option<String>,
    #[serde(default)]
    enabled_start_time: bool,
    #[serde(default)]
    enabled_end_time: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbdmCategory {
    id: i64,
    name: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    path: String,
    #[serde(default = "yes")]
    use_path: bool,
    #[serde(default)]
    accepted_file_types: Vec<String>,
    #[serde(default)]
    accepted_url_patterns: Vec<String>,
    #[serde(default)]
    items: Vec<i64>,
}

fn yes() -> bool {
    true
}

fn json_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

fn map_status(status: Option<&str>) -> Status {
    match status {
        Some("Completed") => Status::Completed,
        Some("Error") => Status::Failed,
        Some("Added") => Status::Added,
        _ => Status::Paused,
    }
}

fn convert_item(raw: AbdmItem, append_extension: bool) -> DownloadItem {
    let status = map_status(raw.status.as_deref());
    let mut item = DownloadItem::new(raw.link, raw.folder, raw.name);
    item.id = raw.id;
    item.kind = if raw.kind.as_deref() == Some("hls") { SourceKind::Hls } else { SourceKind::Http };
    item.headers = raw.headers.map(|h| h.into_iter().collect()).unwrap_or_default();
    item.basic_auth = match (raw.username, raw.password) {
        (Some(user), Some(pass)) if !user.is_empty() => Some((user, pass)),
        _ => None,
    };
    item.referrer = raw.download_page.filter(|s| !s.is_empty());
    item.user_agent = raw.user_agent.filter(|s| !s.is_empty());
    item.name_locked = true;
    item.size = raw.content_length.filter(|n| *n >= 0).map(|n| n as u64);
    item.status = status;
    item.date_added = raw.date_added.unwrap_or(item.date_added);
    item.date_started = raw.start_time;
    item.date_finished = raw.complete_time;
    item.threads = raw.preferred_connection_count.filter(|n| *n > 0).map(|n| n as u32);
    item.speed_limit = raw.speed_limit.unwrap_or(0).max(0) as u64;
    item.checksum = raw.file_checksum.filter(|s| !s.is_empty());
    if status == Status::Completed {
        item.downloaded = item.size.unwrap_or(0);
    }
    if status != Status::Completed {
        item.resume = Some(ResumeState {
            file_name: item.name.clone(),
            working_path: {
                let dir = PathBuf::from(&item.folder);
                if append_extension { dir.join(format!("{}.dl-{}.abdm.part", item.name, item.id)) } else { dir.join(&item.name) }
            },
            total: item.size,
            resumable: true,
            validator: sdm_engine::Validator { etag: raw.server_etag.filter(|s| !s.is_empty()), last_modified: None },
            parts: Vec::new(),
            hls_next_segment: None,
        });
    }
    item
}

/// Reads an ABDM data folder. Unreadable files are reported, never fatal.
pub fn read(config: &Path) -> Result<AbdmData> {
    let Some(config) = locate(config) else {
        bail!("this does not look like an AB Download Manager data folder");
    };
    let mut data = AbdmData::default();

    let settings_file = config.join("appSettings.json");
    if settings_file.is_file() {
        match read_json(&settings_file) {
            Ok(Value::Object(map)) => data.settings = Some(map),
            Ok(_) => data.errors.push("appSettings.json: unexpected shape".into()),
            Err(e) => data.errors.push(e),
        }
    }
    let append_extension =
        data.settings.as_ref().and_then(|s| s.get("appendExtensionToIncompleteDownloads")).and_then(Value::as_bool).unwrap_or(false);

    let db = config.join("download_db");
    let mut by_id: HashMap<i64, usize> = HashMap::new();
    for file in json_files(&db.join("downloadlist")) {
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let parsed = read_json(&file).and_then(|v| parse::<AbdmItem>(v, &name));
        match parsed {
            Ok(raw) => {
                let mut item = convert_item(raw, append_extension);
                if let Some(resume) = item.resume.as_mut() {
                    match read_json(&db.join("parts").join(format!("{}.json", item.id))).and_then(|v| parse::<AbdmParts>(v, "parts")) {
                        Ok(parts) => {
                            resume.parts = parts.list;
                            item.downloaded = resume.downloaded();
                        }
                        Err(_) => item.resume = None,
                    }
                }
                by_id.insert(item.id, data.items.len());
                data.items.push(item);
            }
            Err(e) => data.errors.push(e),
        }
    }

    for file in json_files(&db.join("queues")) {
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match read_json(&file).and_then(|v| parse::<AbdmQueue>(v, &name)) {
            Ok(raw) => {
                let schedule = raw
                    .scheduled_times
                    .map(|s| Schedule {
                        days: if s.days_of_week.is_empty() { Weekday::ALL.to_vec() } else { s.days_of_week },
                        start: s.start_time,
                        end: s.end_time,
                        enabled_start: s.enabled_start_time,
                        enabled_end: s.enabled_end_time,
                    })
                    .unwrap_or_default();
                for id in &raw.queue_items {
                    if let Some(&index) = by_id.get(id) {
                        data.items[index].queue_id = Some(raw.id);
                    }
                }
                data.queues.push(Queue {
                    id: raw.id,
                    name: raw.name,
                    max_concurrent: raw.max_concurrent.unwrap_or(2),
                    items: raw.queue_items,
                    schedule,
                    stop_on_empty: raw.stop_queue_on_empty,
                    power_action: crate::model::PowerAction::None,
                });
            }
            Err(e) => data.errors.push(e),
        }
    }

    let categories_file = db.join("categories").join("categories.json");
    if categories_file.is_file() {
        match read_json(&categories_file).and_then(|v| parse::<Vec<AbdmCategory>>(v, "categories.json")) {
            Ok(list) => {
                for raw in list {
                    for id in &raw.items {
                        if let Some(&index) = by_id.get(id) {
                            data.items[index].category_id = Some(raw.id);
                        }
                    }
                    data.categories.push(Category {
                        id: raw.id,
                        name: raw.name,
                        icon: raw.icon,
                        folder: raw.path,
                        use_folder: raw.use_path,
                        extensions: raw.accepted_file_types,
                        url_patterns: raw.accepted_url_patterns,
                    });
                }
            }
            Err(e) => data.errors.push(e),
        }
    }
    Ok(data)
}

fn unit(value: &Value) -> Option<UnitMode> {
    serde_json::from_value(value.clone()).ok()
}

/// Copies the ABDM options that matter here. The api key and port are never copied.
pub fn apply_settings(data: &AbdmData, settings: &mut Settings) {
    let Some(map) = &data.settings else { return };
    let text = |key: &str| map.get(key).and_then(Value::as_str).map(str::to_owned);
    let flag = |key: &str| map.get(key).and_then(Value::as_bool);
    let number = |key: &str| map.get(key).and_then(Value::as_i64).filter(|n| *n >= 0);

    if let Some(v) = text("defaultDownloadFolder").filter(|s| !s.is_empty()) {
        settings.default_download_folder = v;
    }
    if let Some(v) = number("threadCount").filter(|n| *n > 0) {
        settings.thread_count = v as u32;
    }
    if let Some(v) = number("maxConcurrentDownloads") {
        settings.max_concurrent_downloads = v as u32;
    }
    if let Some(v) = number("maxDownloadRetryCount") {
        settings.max_retry_count = v as u32;
    }
    if let Some(v) = number("minPartSize").filter(|n| *n > 0) {
        settings.min_part_size = v as u64;
    }
    if let Some(v) = number("speedLimit") {
        settings.speed_limit = v as u64;
    }
    if let Some(mode) = map.get("useSpeedLimit").and_then(|v| serde_json::from_value::<SpeedLimitMode>(v.clone()).ok()) {
        settings.speed_limit_mode = mode;
    }
    if let Some(v) = map.get("sizeUnit").and_then(unit) {
        settings.size_unit = v;
    }
    if let Some(v) = map.get("speedUnit").and_then(unit) {
        settings.speed_unit = v;
    }
    if let Some(v) = text("userAgent") {
        settings.user_agent = v;
    }
    if let Some(v) = text("font") {
        settings.font = Some(v);
    }
    if let Some(v) = text("language") {
        settings.language = Some(v);
    }
    if let Some(v) = map.get("uiScale").and_then(Value::as_f64) {
        settings.ui_scale = Some(v as f32);
    }
    for (key, target) in [
        ("showIconLabels", &mut settings.show_icon_labels),
        ("useRelativeDateTime", &mut settings.use_relative_date_time),
        ("dynamicPartCreation", &mut settings.dynamic_part_creation),
        ("useServerLastModifiedTime", &mut settings.use_server_last_modified_time),
        ("appendExtensionToIncompleteDownloads", &mut settings.append_extension_to_incomplete_downloads),
        ("useSparseFileAllocation", &mut settings.use_sparse_file_allocation),
        ("useAverageSpeed", &mut settings.use_average_speed),
        ("showDownloadProgressDialog", &mut settings.show_download_progress_dialog),
        ("showDownloadCompletionDialog", &mut settings.show_download_completion_dialog),
        ("autoStartOnBoot", &mut settings.auto_start_on_boot),
        ("notificationSound", &mut settings.notification_sound),
        ("trackDeletedFilesOnDisk", &mut settings.track_deleted_files_on_disk),
        ("deletePartialFileOnDownloadCancellation", &mut settings.delete_partial_file_on_cancellation),
        ("ignoreSSLCertificates", &mut settings.ignore_ssl_certificates),
        ("useCategoryByDefault", &mut settings.use_category_by_default),
        ("blockAddWhenLoading", &mut settings.block_add_when_loading),
        ("useSystemTray", &mut settings.use_system_tray),
    ] {
        if let Some(v) = flag(key) {
            *target = v;
        }
    }
    for (key, target) in [
        ("generalNotificationSound", &mut settings.general_notification_sound),
        ("errorNotificationSound", &mut settings.error_notification_sound),
        ("successNotificationSound", &mut settings.success_notification_sound),
    ] {
        if let Some(v) = text(key) {
            *target = v;
        }
    }
}

fn same_category_shape(a: &Category, b: &Category) -> bool {
    a.id == b.id && a.name == b.name && a.extensions == b.extensions && a.url_patterns == b.url_patterns
}

/// Writes the parsed data into the store, keeping ABDM ids. Existing ids are skipped.
pub fn apply(data: &AbdmData, store: &Store) -> Result<ImportReport> {
    let mut report = ImportReport { errors: data.errors.clone(), ..Default::default() };

    for item in &data.items {
        if store.download_exists(item.id)? {
            report.skipped += 1;
        } else {
            store.insert_download(item)?;
            report.downloads += 1;
        }
    }

    let existing_queues = store.load_queues()?;
    for queue in &data.queues {
        match existing_queues.iter().find(|q| q.id == queue.id) {
            Some(existing) if !existing.items.is_empty() => report.skipped += 1,
            _ => {
                store.save_queue(queue)?;
                report.queues += 1;
            }
        }
    }

    let existing_categories = store.load_categories()?;
    let defaults = default_categories(Path::new(""));
    for category in &data.categories {
        match existing_categories.iter().find(|c| c.id == category.id) {
            Some(existing) if !defaults.iter().any(|d| same_category_shape(d, existing)) => report.skipped += 1,
            _ => {
                store.save_category(category)?;
                report.categories += 1;
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("abdm")
    }

    #[test]
    fn reads_fixture() {
        let data = read(&fixture()).unwrap();
        assert_eq!(data.items.len(), 3);
        assert_eq!(data.queues.len(), 1);
        assert!(data.categories.is_empty());
        assert_eq!(data.errors.len(), 1, "the empty download file is reported: {:?}", data.errors);

        let done = data.items.iter().find(|i| i.id == 1).unwrap();
        assert_eq!(done.status, Status::Completed);
        assert_eq!(done.downloaded, 2048);
        assert_eq!(done.headers.len(), 1);
        assert_eq!(done.referrer.as_deref(), Some("https://example.com/page"));
        assert!(done.resume.is_none());

        let paused = data.items.iter().find(|i| i.id == 2).unwrap();
        assert_eq!(paused.status, Status::Paused);
        assert_eq!(paused.queue_id, Some(0));
        let resume = paused.resume.as_ref().unwrap();
        assert_eq!(resume.parts.len(), 2);
        assert_eq!(paused.downloaded, 150 + 100);
        assert!(resume.working_path.to_string_lossy().ends_with("big.bin.dl-2.abdm.part"));

        let failed = data.items.iter().find(|i| i.id == 4).unwrap();
        assert_eq!(failed.status, Status::Failed);
        assert_eq!(failed.basic_auth, Some(("fakeuser".into(), "fakepass".into())));
    }

    #[test]
    fn settings_never_copy_api_key() {
        let data = read(&fixture()).unwrap();
        let mut settings = Settings { api_key: "mine".into(), api_port: 15161, ..Settings::default() };
        apply_settings(&data, &mut settings);
        assert_eq!(settings.api_key, "mine");
        assert_eq!(settings.api_port, 15161);
        assert_eq!(settings.thread_count, 6);
        assert_eq!(settings.default_download_folder, "D:\\Fixture Downloads");
        assert_eq!(settings.size_unit, UnitMode::DecimalBytes);
        assert!(!settings.use_category_by_default);
        assert_eq!(settings.speed_limit, 262140);
        assert_eq!(settings.max_concurrent_downloads, 3);
    }

    #[test]
    fn apply_is_idempotent() {
        let data = read(&fixture()).unwrap();
        let store = Store::open_memory().unwrap();
        store.save_queue(&Queue::main()).unwrap();
        let first = apply(&data, &store).unwrap();
        assert_eq!((first.downloads, first.queues, first.skipped), (3, 1, 0));
        let second = apply(&data, &store).unwrap();
        assert_eq!(second.downloads, 0);
        assert_eq!(second.skipped, 3 + 1);
        assert_eq!(store.load_downloads().unwrap().len(), 3);
        assert_eq!(store.load_queues().unwrap()[0].items, vec![2]);
    }

    #[test]
    fn rejects_unrelated_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).is_err());
    }

    #[test]
    fn error_messages_hold_no_values() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let list = config.join("download_db").join("downloadlist");
        fs::create_dir_all(&list).unwrap();
        fs::write(list.join("9.json"), r#"{"link": 5, "id": "secret-cookie-value"}"#).unwrap();
        let data = read(&config).unwrap();
        assert_eq!(data.errors.len(), 1);
        assert!(!data.errors[0].contains("secret-cookie-value"));
    }
}
