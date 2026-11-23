use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW,
};
use windows::core::PCWSTR;

use crate::ids::{CHROMIUM_EXTENSION_ID, FIREFOX_EXTENSION_ID, HOST_NAME};

// Helium reads its own vendor key; the plain Chromium key covers other ungoogled builds.
const CHROMIUM_KEYS: [&str; 4] =
    ["Software\\Google\\Chrome", "Software\\BraveSoftware\\Brave-Browser", "Software\\Chromium", "Software\\imput\\Helium"];
const FIREFOX_KEY: &str = "Software\\Mozilla";

pub fn data_dir() -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").ok_or_else(|| anyhow!("LOCALAPPDATA is not set"))?;
    Ok(PathBuf::from(base).join("SeamlessDownloadManager"))
}

fn manifest_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("native-messaging"))
}

pub fn chromium_manifest(host_exe: &Path, extension_id: &str) -> Value {
    json!({
        "name": HOST_NAME,
        "description": "Seamless Download Manager",
        "path": host_exe.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{extension_id}/")],
    })
}

pub fn firefox_manifest(host_exe: &Path, extension_id: &str) -> Value {
    json!({
        "name": HOST_NAME,
        "description": "Seamless Download Manager",
        "path": host_exe.to_string_lossy(),
        "type": "stdio",
        "allowed_extensions": [extension_id],
    })
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn set_default_value(subkey: &str, value: &str) -> Result<()> {
    let key_name = wide(subkey);
    let data = wide(value);
    let bytes: Vec<u8> = data.iter().flat_map(|u| u.to_le_bytes()).collect();
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(HKEY_CURRENT_USER, PCWSTR(key_name.as_ptr()), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None)
            .ok()
            .with_context(|| format!("could not create HKCU\\{subkey}"))?;
        let set = RegSetValueExW(key, PCWSTR::null(), None, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(key);
        set.ok().with_context(|| format!("could not write HKCU\\{subkey}"))?;
    }
    Ok(())
}

fn delete_tree(subkey: &str) {
    let key_name = wide(subkey);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(key_name.as_ptr()));
    }
}

fn host_keys() -> Vec<(String, bool)> {
    let mut keys: Vec<(String, bool)> = CHROMIUM_KEYS.iter().map(|k| (format!("{k}\\NativeMessagingHosts\\{HOST_NAME}"), false)).collect();
    keys.push((format!("{FIREFOX_KEY}\\NativeMessagingHosts\\{HOST_NAME}"), true));
    keys
}

fn chromium_path(dir: &Path) -> PathBuf {
    dir.join(format!("{HOST_NAME}.chromium.json"))
}

fn firefox_path(dir: &Path) -> PathBuf {
    dir.join(format!("{HOST_NAME}.firefox.json"))
}

pub fn install() -> Result<()> {
    let exe = std::env::current_exe()?;
    let dir = manifest_dir()?;
    std::fs::create_dir_all(&dir)?;
    let chromium = chromium_path(&dir);
    let firefox = firefox_path(&dir);
    std::fs::write(&chromium, serde_json::to_vec_pretty(&chromium_manifest(&exe, CHROMIUM_EXTENSION_ID))?)?;
    std::fs::write(&firefox, serde_json::to_vec_pretty(&firefox_manifest(&exe, FIREFOX_EXTENSION_ID))?)?;
    for (key, is_firefox) in host_keys() {
        let file = if is_firefox { &firefox } else { &chromium };
        set_default_value(&key, &file.to_string_lossy())?;
    }
    Ok(())
}

pub fn uninstall() -> Result<()> {
    for (key, _) in host_keys() {
        delete_tree(&key);
    }
    let dir = manifest_dir()?;
    let _ = std::fs::remove_file(chromium_path(&dir));
    let _ = std::fs::remove_file(firefox_path(&dir));
    let _ = std::fs::remove_dir(&dir);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_carry_the_right_origins() {
        let exe = Path::new("C:\\Apps\\sdm-host.exe");
        let chromium = chromium_manifest(exe, "abcdefgh");
        assert_eq!(chromium["allowed_origins"][0], "chrome-extension://abcdefgh/");
        assert_eq!(chromium["path"], "C:\\Apps\\sdm-host.exe");
        assert_eq!(chromium["type"], "stdio");
        assert!(chromium.get("allowed_extensions").is_none());
        let firefox = firefox_manifest(exe, FIREFOX_EXTENSION_ID);
        assert_eq!(firefox["allowed_extensions"][0], "sdm@seamless.local");
        assert!(firefox.get("allowed_origins").is_none());
    }

    #[test]
    fn registry_paths_cover_every_browser() {
        let keys = host_keys();
        assert_eq!(keys.len(), 5);
        assert!(keys.iter().all(|(k, _)| k.ends_with("NativeMessagingHosts\\com.seamless.download_manager")));
        assert_eq!(keys.iter().filter(|(_, firefox)| *firefox).count(), 1);
    }
}
