use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use sdm_engine::{ResumeState, SourceKind};

use crate::util::wildcard_match;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Added,
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Retrying,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Added => "added",
            Status::Queued => "queued",
            Status::Downloading => "downloading",
            Status::Paused => "paused",
            Status::Completed => "completed",
            Status::Failed => "failed",
            Status::Retrying => "retrying",
        }
    }

    pub fn parse(value: &str) -> Option<Status> {
        Some(match value {
            "added" => Status::Added,
            "queued" => Status::Queued,
            "downloading" => Status::Downloading,
            "paused" => Status::Paused,
            "completed" => Status::Completed,
            "failed" => Status::Failed,
            "retrying" => Status::Retrying,
            _ => return None,
        })
    }

    pub fn is_running(self) -> bool {
        matches!(self, Status::Downloading | Status::Retrying)
    }
}

#[derive(Clone)]
pub struct DownloadItem {
    pub id: i64,
    pub url: String,
    pub kind: SourceKind,
    pub headers: Vec<(String, String)>,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub basic_auth: Option<(String, String)>,
    pub folder: String,
    pub name: String,
    /// True when the user or the browser chose the name, so the server may not rename it.
    pub name_locked: bool,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub status: Status,
    pub error: Option<String>,
    pub date_added: i64,
    pub date_started: Option<i64>,
    pub date_finished: Option<i64>,
    pub queue_id: Option<i64>,
    pub category_id: Option<i64>,
    pub threads: Option<u32>,
    pub speed_limit: u64,
    pub checksum: Option<String>,
    pub resume: Option<ResumeState>,
}

impl fmt::Debug for DownloadItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DownloadItem")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

impl DownloadItem {
    pub fn new(url: impl Into<String>, folder: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: 0,
            url: url.into(),
            kind: SourceKind::Http,
            headers: Vec::new(),
            referrer: None,
            user_agent: None,
            basic_auth: None,
            folder: folder.into(),
            name: name.into(),
            name_locked: false,
            size: None,
            downloaded: 0,
            status: Status::Added,
            error: None,
            date_added: crate::util::now_ms(),
            date_started: None,
            date_finished: None,
            queue_id: None,
            category_id: None,
            threads: None,
            speed_limit: 0,
            checksum: None,
            resume: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];

    pub fn from_chrono(day: chrono::Weekday) -> Weekday {
        Weekday::ALL[day.num_days_from_monday() as usize]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Schedule {
    pub days: Vec<Weekday>,
    /// "HH:MM", 24 hour.
    pub start: Option<String>,
    pub end: Option<String>,
    pub enabled_start: bool,
    pub enabled_end: bool,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            days: Weekday::ALL.to_vec(),
            start: Some("02:30".into()),
            end: Some("07:30".into()),
            enabled_start: false,
            enabled_end: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Queue {
    pub id: i64,
    pub name: String,
    pub max_concurrent: u32,
    /// Download ids in start order.
    pub items: Vec<i64>,
    pub schedule: Schedule,
    pub stop_on_empty: bool,
    /// What the computer does once this queue finishes everything in it.
    #[serde(default)]
    pub power_action: PowerAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PowerAction {
    #[default]
    None,
    Sleep,
    Hibernate,
    Shutdown,
    Restart,
}

impl PowerAction {
    /// In the order the queue page lists them.
    pub const ALL: [PowerAction; 5] =
        [PowerAction::None, PowerAction::Sleep, PowerAction::Hibernate, PowerAction::Shutdown, PowerAction::Restart];

    pub fn as_str(self) -> &'static str {
        match self {
            PowerAction::None => "none",
            PowerAction::Sleep => "sleep",
            PowerAction::Hibernate => "hibernate",
            PowerAction::Shutdown => "shutdown",
            PowerAction::Restart => "restart",
        }
    }

    pub fn parse(value: &str) -> PowerAction {
        PowerAction::ALL.into_iter().find(|a| a.as_str() == value).unwrap_or_default()
    }

    pub fn index(self) -> usize {
        PowerAction::ALL.iter().position(|a| *a == self).unwrap_or(0)
    }
}

pub const MAIN_QUEUE_ID: i64 = 0;

impl Queue {
    pub fn main() -> Queue {
        Queue {
            id: MAIN_QUEUE_ID,
            name: "Main".into(),
            max_concurrent: 2,
            items: Vec::new(),
            schedule: Schedule::default(),
            stop_on_empty: false,
            power_action: PowerAction::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub folder: String,
    pub use_folder: bool,
    pub extensions: Vec<String>,
    pub url_patterns: Vec<String>,
}

impl Category {
    pub fn matches(&self, file_name: &str, url: &str) -> bool {
        let lower = file_name.to_ascii_lowercase();
        let by_extension = self.extensions.iter().any(|ext| {
            let ext = ext.trim_start_matches('.').to_ascii_lowercase();
            !ext.is_empty() && lower.ends_with(&format!(".{ext}"))
        });
        by_extension || self.url_patterns.iter().any(|pattern| wildcard_match(pattern, url))
    }
}

pub fn default_categories(default_folder: &Path) -> Vec<Category> {
    let table: [(i64, &str, &str, &[&str]); 6] = [
        (0, "Compressed", "zip", &["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "tgz"]),
        (1, "Programs", "application", &["apk", "exe", "msi", "bat", "sh", "jar", "app", "deb", "rpm", "bin"]),
        (2, "Videos", "video", &["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "3gp", "mpeg", "ts"]),
        (3, "Music", "music", &["mp3", "wav", "aac", "flac", "ogg", "aiff", "wma", "m4a"]),
        (
            4,
            "Pictures",
            "picture",
            &["jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "svg", "webp", "heic", "ico", "raw", "psd"],
        ),
        (
            5,
            "Documents",
            "document",
            &["doc", "docx", "pdf", "txt", "rtf", "odt", "xls", "xlsx", "ppt", "pptx", "csv", "epub", "pages"],
        ),
    ];
    table
        .iter()
        .map(|(id, name, icon, extensions)| Category {
            id: *id,
            name: (*name).into(),
            icon: (*icon).into(),
            folder: default_folder.join(name).to_string_lossy().into_owned(),
            use_folder: true,
            extensions: extensions.iter().map(|e| (*e).into()).collect(),
            url_patterns: Vec::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_default_categories_with_abdm_lists() {
        let cats = default_categories(Path::new("D:\\Dl"));
        assert_eq!(cats.len(), 6);
        assert_eq!(cats[2].name, "Videos");
        assert!(cats[2].extensions.contains(&"ts".to_string()));
        assert_eq!(cats[5].extensions.len(), 13);
        assert_eq!(cats[0].folder, "D:\\Dl\\Compressed");
    }

    #[test]
    fn category_matches_extension_and_pattern() {
        let mut cat = default_categories(Path::new("D:\\Dl")).remove(0);
        assert!(cat.matches("Archive.ZIP", "https://x/y"));
        assert!(!cat.matches("movie.mp4", "https://x/y"));
        cat.url_patterns.push("https://files.example.com/*".into());
        assert!(cat.matches("noext", "https://files.example.com/a/b"));
    }

    #[test]
    fn status_round_trips() {
        for status in [Status::Added, Status::Queued, Status::Downloading, Status::Paused, Status::Completed, Status::Failed, Status::Retrying] {
            assert_eq!(Status::parse(status.as_str()), Some(status));
        }
    }
}
