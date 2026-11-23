use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sdm_engine::ProxyConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UnitMode {
    #[default]
    BinaryBytes,
    DecimalBytes,
    BinaryBits,
    DecimalBits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SpeedLimitMode {
    #[default]
    Disabled,
    Enabled,
    #[serde(alias = "EnabledWhenBessy")]
    EnabledWhenBusy,
}

/// Settings for links from one site. `host` may hold `*`; the rule with the fewest `*` wins.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PerHostRule {
    pub host: String,
    pub username: String,
    pub password: String,
    pub user_agent: String,
    /// 0 keeps the global setting.
    pub threads: u32,
    /// Bytes per second, 0 = none.
    pub speed_limit: u64,
}

impl fmt::Debug for PerHostRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PerHostRule").field("host", &self.host).finish_non_exhaustive()
    }
}

impl PerHostRule {
    pub fn matches(&self, host: &str) -> bool {
        let pattern = self.host.trim();
        !pattern.is_empty() && crate::util::wildcard_match(pattern, host)
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub font: Option<String>,
    pub ui_scale: Option<f32>,
    pub language: Option<String>,
    pub show_icon_labels: bool,
    pub use_relative_date_time: bool,
    pub thread_count: u32,
    /// 0 means unlimited.
    pub max_concurrent_downloads: u32,
    pub max_retry_count: u32,
    pub dynamic_part_creation: bool,
    pub use_server_last_modified_time: bool,
    pub append_extension_to_incomplete_downloads: bool,
    pub temp_suffix: String,
    pub use_sparse_file_allocation: bool,
    pub use_average_speed: bool,
    pub show_download_progress_dialog: bool,
    pub show_download_completion_dialog: bool,
    pub speed_limit_mode: SpeedLimitMode,
    /// Bytes per second.
    pub speed_limit: u64,
    pub min_part_size: u64,
    pub auto_start_on_boot: bool,
    pub notification_sound: bool,
    pub general_notification_sound: String,
    pub error_notification_sound: String,
    pub success_notification_sound: String,
    pub default_download_folder: String,
    pub track_deleted_files_on_disk: bool,
    pub delete_partial_file_on_cancellation: bool,
    pub size_unit: UnitMode,
    pub speed_unit: UnitMode,
    pub ignore_ssl_certificates: bool,
    pub use_category_by_default: bool,
    pub user_agent: String,
    pub block_add_when_loading: bool,
    pub use_system_tray: bool,
    pub api_enabled: bool,
    pub api_port: u16,
    pub api_key: String,
    pub window_tint: f32,
    pub start_minimized_to_tray: bool,
    pub proxy: ProxyConfig,
    /// Per-site rules, checked when a link is added.
    pub per_host: Vec<PerHostRule>,
    /// Empty: Windows looks up names. Otherwise an https address that answers DNS questions.
    pub dns_over_https: String,
}

impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings").field("api_port", &self.api_port).finish_non_exhaustive()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "catppuccin_mocha".into(),
            font: None,
            ui_scale: None,
            language: None,
            show_icon_labels: true,
            use_relative_date_time: true,
            thread_count: 8,
            max_concurrent_downloads: 3,
            max_retry_count: 3,
            dynamic_part_creation: true,
            use_server_last_modified_time: false,
            append_extension_to_incomplete_downloads: true,
            temp_suffix: ".sdm-part".into(),
            use_sparse_file_allocation: true,
            use_average_speed: true,
            show_download_progress_dialog: false,
            show_download_completion_dialog: false,
            speed_limit_mode: SpeedLimitMode::Disabled,
            speed_limit: 262_144,
            min_part_size: 524_288,
            auto_start_on_boot: false,
            notification_sound: true,
            general_notification_sound: String::new(),
            error_notification_sound: String::new(),
            success_notification_sound: String::new(),
            default_download_folder: default_download_folder(),
            track_deleted_files_on_disk: false,
            delete_partial_file_on_cancellation: true,
            size_unit: UnitMode::BinaryBytes,
            speed_unit: UnitMode::BinaryBytes,
            ignore_ssl_certificates: false,
            use_category_by_default: true,
            user_agent: String::new(),
            block_add_when_loading: true,
            use_system_tray: true,
            api_enabled: true,
            api_port: 15161,
            api_key: String::new(),
            window_tint: 0.45,
            start_minimized_to_tray: false,
            proxy: ProxyConfig::System,
            per_host: Vec::new(),
            dns_over_https: String::new(),
        }
    }
}

fn default_download_folder() -> String {
    let base = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("Downloads").join("SDM").to_string_lossy().into_owned()
}

pub fn generate_api_key() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut key = String::with_capacity(32);
    let mut buf = [0u8; 64];
    while key.len() < 32 {
        getrandom::fill(&mut buf).expect("system random source");
        for byte in buf {
            if byte < 248 && key.len() < 32 {
                key.push(ALPHABET[(byte % 62) as usize] as char);
            }
        }
    }
    key
}

impl Settings {
    /// Reads the file, falling back to defaults per missing key. A missing file or empty key
    /// gets a fresh api key and is written back.
    pub fn load(path: &Path) -> Result<Settings> {
        let mut settings = match fs::read_to_string(path) {
            Ok(text) if !text.trim().is_empty() => {
                serde_json::from_str(text.trim_start_matches('\u{feff}')).with_context(|| "settings.json is not valid")?
            }
            Ok(_) => Settings::default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => return Err(e).context("could not read settings.json"),
        };
        if settings.api_key.trim().is_empty() {
            settings.api_key = generate_api_key();
            settings.save(path)?;
        }
        Ok(settings)
    }

    /// Pretty JSON through a temp file and a rename.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        fs::rename(&tmp, path).context("could not replace settings.json")?;
        Ok(())
    }

    pub fn effective_speed_limit(&self) -> u64 {
        match self.speed_limit_mode {
            SpeedLimitMode::Disabled => 0,
            _ => self.speed_limit,
        }
    }

    pub fn default_folder(&self) -> PathBuf {
        PathBuf::from(&self.default_download_folder)
    }

    /// The rule for a link, if any: fewest `*` first, then the longest fixed text, then list order.
    pub fn rule_for(&self, url: &str) -> Option<&PerHostRule> {
        let host = url::Url::parse(url).ok()?.host_str()?.to_ascii_lowercase();
        let mut rules: Vec<&PerHostRule> = self.per_host.iter().filter(|r| r.matches(&host)).collect();
        rules.sort_by_key(|r| {
            let stars = r.host.matches('*').count();
            (stars, std::cmp::Reverse(r.host.trim().len() - stars))
        });
        rules.into_iter().next()
    }

    /// The DNS-over-HTTPS address in use, or None for the system resolver.
    pub fn doh_url(&self) -> Option<String> {
        let url = self.dns_over_https.trim();
        (!url.is_empty()).then(|| url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_generates_key_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let first = Settings::load(&path).unwrap();
        assert_eq!(first.api_key.len(), 32);
        assert!(first.api_key.chars().all(|c| c.is_ascii_alphanumeric()));
        let second = Settings::load(&path).unwrap();
        assert_eq!(first.api_key, second.api_key);
        assert_eq!(second.api_port, 15161);
    }

    #[test]
    fn missing_keys_fall_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"thread_count": 4, "api_key": "abc"}"#).unwrap();
        let s = Settings::load(&path).unwrap();
        assert_eq!(s.thread_count, 4);
        assert_eq!(s.temp_suffix, ".sdm-part");
        assert_eq!(s.api_key, "abc");
    }

    #[test]
    fn save_is_atomic_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = Settings { api_key: "k".into(), proxy: ProxyConfig::Manual { url: "http://p:1".into() }, ..Settings::default() };
        s.save(&path).unwrap();
        assert!(!dir.path().join("settings.json.tmp").exists());
        assert_eq!(Settings::load(&path).unwrap(), s);
    }

    #[test]
    fn per_host_prefers_fewest_wildcards() {
        let rule = |host: &str, threads| PerHostRule { host: host.into(), threads, ..Default::default() };
        let s = Settings {
            per_host: vec![rule("*", 1), rule("*.example.com", 2), rule("files.example.com", 3), rule("", 9)],
            ..Settings::default()
        };
        assert_eq!(s.rule_for("https://files.example.com/a.zip").unwrap().threads, 3);
        assert_eq!(s.rule_for("https://cdn.Example.com/a.zip").unwrap().threads, 2);
        assert_eq!(s.rule_for("https://other.net/a.zip").unwrap().threads, 1);
        assert!(Settings::default().rule_for("https://other.net/").is_none());
        assert!(s.rule_for("not a link").is_none());
    }

    #[test]
    fn keys_are_unique() {
        assert_ne!(generate_api_key(), generate_api_key());
    }
}
