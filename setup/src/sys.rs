//! The Windows pieces of the installer: lock check, registry, Start menu shortcut, glass.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, IPersistFile,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Shell::{FOLDERID_Programs, IShellLinkW, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath, ShellLink};
use windows::core::{HSTRING, Interface, PCWSTR, w};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const INSTANCE: PCWSTR = w!("Local\\SeamlessDownloadManager");
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("SeamlessDownloadManager");
const UNINSTALL_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SeamlessDownloadManager");

/// True while the app or the Windhawk mod's engine holds the shared lock.
pub fn instance_running() -> bool {
    unsafe {
        match OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, INSTANCE) {
            Ok(h) => {
                let _ = CloseHandle(h);
                true
            }
            Err(_) => false,
        }
    }
}

pub fn run_hidden(exe: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .with_context(|| format!("could not run {}", exe.display()))?;
    if !status.success() {
        bail!("{} {} ended with code {}", exe.display(), args.join(" "), status.code().unwrap_or(-1));
    }
    Ok(())
}

pub fn launch(exe: &Path, args: &[&str]) -> Result<()> {
    Command::new(exe)
        .args(args)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .spawn()
        .with_context(|| format!("could not start {}", exe.display()))?;
    Ok(())
}

pub fn start_menu_shortcut() -> Result<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Programs, KNOWN_FOLDER_FLAG(0), None)
            .context("could not find the Start menu folder")?;
        let dir = PathBuf::from(p.to_string().unwrap_or_default());
        CoTaskMemFree(Some(p.0 as *const _));
        if dir.as_os_str().is_empty() {
            bail!("could not find the Start menu folder");
        }
        Ok(dir.join("Seamless Download Manager.lnk"))
    }
}

pub fn make_shortcut(link: &Path, target: &Path) -> Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let sl: IShellLinkW =
            CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).context("could not make a shortcut")?;
        sl.SetPath(&HSTRING::from(target.as_os_str()))?;
        if let Some(dir) = target.parent() {
            sl.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        }
        sl.SetIconLocation(&HSTRING::from(target.as_os_str()), 0)?;
        sl.SetDescription(w!("Download files from your browser, fast."))?;
        let file: IPersistFile = sl.cast()?;
        file.Save(&HSTRING::from(link.as_os_str()), true).context("could not save the Start menu shortcut")?;
    }
    Ok(())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe fn set_sz(key: HKEY, name: &str, value: &str) -> Result<()> {
    let v = wide(value);
    let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2) };
    unsafe { RegSetValueExW(key, &HSTRING::from(name), None, REG_SZ, Some(bytes)) }.ok()?;
    Ok(())
}

unsafe fn set_dword(key: HKEY, name: &str, value: u32) -> Result<()> {
    unsafe { RegSetValueExW(key, &HSTRING::from(name), None, REG_DWORD, Some(&value.to_le_bytes())) }.ok()?;
    Ok(())
}

/// `Some(exe)` starts the app in the tray at sign-in, `None` stops that.
pub fn set_autostart(exe: Option<&Path>) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_SET_VALUE, &mut key).ok().context("could not open the startup list")?;
        let result = match exe {
            Some(exe) => set_sz(key, "SeamlessDownloadManager", &format!("\"{}\" --background", exe.display())),
            None => {
                let r = RegDeleteValueW(key, RUN_VALUE);
                if r.is_err() && r.0 != 2 { r.ok().map_err(Into::into) } else { Ok(()) }
            }
        };
        let _ = RegCloseKey(key);
        result.context("could not change start with Windows")
    }
}

/// The entry in Settings > Apps.
pub fn register_uninstall(bin: &Path, app: &Path, uninstaller: &Path, version: &str, size_kb: u32) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(HKEY_CURRENT_USER, UNINSTALL_KEY, None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None)
            .ok()
            .context("could not add the app to Settings > Apps")?;
        let u = uninstaller.display();
        let result = (|| -> Result<()> {
            set_sz(key, "DisplayName", "Seamless Download Manager")?;
            set_sz(key, "DisplayVersion", version)?;
            set_sz(key, "DisplayIcon", &format!("{},0", app.display()))?;
            set_sz(key, "Publisher", "DavidHiFi")?;
            set_sz(key, "InstallLocation", &bin.display().to_string())?;
            set_sz(key, "UninstallString", &format!("\"{u}\" --uninstall"))?;
            set_sz(key, "QuietUninstallString", &format!("\"{u}\" --uninstall --silent"))?;
            set_sz(key, "URLInfoAbout", "https://github.com/DavidHiFi/seamless-download-manager")?;
            set_dword(key, "EstimatedSize", size_kb)?;
            set_dword(key, "NoModify", 1)?;
            set_dword(key, "NoRepair", 1)?;
            Ok(())
        })();
        let _ = RegCloseKey(key);
        result
    }
}

pub fn unregister_uninstall() -> Result<()> {
    let r = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, UNINSTALL_KEY) };
    if r.is_err() && r.0 != 2 {
        r.ok().context("could not remove the app from Settings > Apps")?;
    }
    Ok(())
}

pub fn installed_version() -> Option<String> {
    let mut buf = [0u16; 64];
    let mut size = (buf.len() * 2) as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            UNINSTALL_KEY,
            w!("DisplayVersion"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
        .ok()
        .ok()?;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
    let v = String::from_utf16_lossy(&buf[..len]);
    (!v.is_empty()).then_some(v)
}

/// Runs the remover again from a temp copy, so the real one can be deleted.
pub fn relaunch_from_temp(args: &[String]) -> Result<()> {
    let me = std::env::current_exe()?;
    let copy = std::env::temp_dir().join(format!("sdm-remove-{}.exe", std::process::id()));
    std::fs::copy(&me, &copy).context("copy")?;
    let mut all: Vec<&str> = args.iter().map(String::as_str).collect();
    all.push("--from-temp");
    let child = Command::new(&copy).args(&all).current_dir(std::env::temp_dir()).spawn()?;
    // A silent remove keeps the caller waiting for the real result.
    if args.iter().any(|a| a == "--silent") {
        let mut child = child;
        let code = child.wait()?.code().unwrap_or(1);
        std::process::exit(code);
    }
    Ok(())
}

/// Deletes files that are still running (this temp copy, the remover the caller started)
/// once their processes have exited. Tries once a second for half a minute.
pub fn delete_later(files: &[PathBuf]) {
    if files.is_empty() {
        return;
    }
    let names: Vec<String> = files.iter().map(|f| format!("\"{}\"", f.display())).collect();
    let all = names.join(" ");
    let gone: String = names.iter().map(|n| format!("if not exist {n} ")).collect();
    let cmd = format!(
        "for /l %i in (1,1,30) do @(ping 127.0.0.1 -n 2 >nul & del /f /q {all} 2>nul & {gone}exit /b 0)"
    );
    // cmd does not read backslash-escaped quotes, so the line goes through untouched.
    let _ = Command::new("cmd").args(["/d", "/c"]).raw_arg(&cmd).creation_flags(CREATE_NO_WINDOW).spawn();
}

/// Dark title bar and the frosted backdrop, as in the app.
pub fn apply_glass(window: &slint::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle();
    let Ok(h) = handle.window_handle() else { return };
    let RawWindowHandle::Win32(h) = h.as_raw() else { return };
    let hwnd = HWND(h.hwnd.get() as *mut _);
    unsafe {
        let dark: i32 = 1;
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(20), &dark as *const i32 as *const _, 4);
        let backdrop: i32 = 3;
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(38), &backdrop as *const i32 as *const _, 4);
        let margins = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }
}
