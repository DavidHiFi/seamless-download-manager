//! The small bits of Windows the window needs: glass, single instance, opening files, start with Windows.

use std::path::Path;

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, IsIconic, RegisterWindowMessageW, SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{HSTRING, PCWSTR, w};

const DWMWA_USE_IMMERSIVE_DARK_MODE: i32 = 20;
const DWMWA_SYSTEMBACKDROP_TYPE: i32 = 38;
const DWMSBT_TRANSIENTWINDOW: i32 = 3;
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("SeamlessDownloadManager");

pub fn hwnd_of(window: &slint::Window) -> Option<HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle();
    let raw = handle.window_handle().ok()?.as_raw();
    match raw {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut _)),
        _ => None,
    }
}

/// Dark title bar plus the acrylic backdrop behind the see-through window body.
pub fn apply_glass(hwnd: HWND) {
    unsafe {
        let dark: i32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_USE_IMMERSIVE_DARK_MODE),
            &dark as *const i32 as *const _,
            4,
        );
        let backdrop = DWMSBT_TRANSIENTWINDOW;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_SYSTEMBACKDROP_TYPE),
            &backdrop as *const i32 as *const _,
            4,
        );
        let margins = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }
}

/// Keeps the window alive when Explorer restarts. Windows then sends every window a "taskbar
/// created" message, and the window library answers it by calling the taskbar while it holds
/// its own lock. The taskbar calls back into the window, which waits on that same lock forever,
/// and Explorer waits on us. We never hide from the taskbar, so the message can skip it.
pub fn guard_taskbar_restart(hwnd: HWND) {
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(taskbar_guard_proc), 0x5D4D, 0);
    }
}

unsafe extern "system" fn taskbar_guard_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    static TASKBAR_CREATED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    let created = *TASKBAR_CREATED.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });
    if msg == created && created != 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Brings the window forward. Only called after the user asked for it (tray click, second launch).
pub fn bring_forward(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

/// Holds the single-instance mutex for the life of the process.
pub struct Instance(#[allow(dead_code)] HANDLE);

/// `None` when another copy already runs.
pub fn claim_instance() -> Option<Instance> {
    unsafe {
        let handle = CreateMutexW(None, true, w!("Local\\SeamlessDownloadManager")).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return None;
        }
        Some(Instance(handle))
    }
}

pub fn shell_open(target: &str) {
    unsafe {
        let target = HSTRING::from(target);
        ShellExecuteW(None, w!("open"), &target, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

pub fn show_in_folder(path: &Path) {
    use std::os::windows::process::CommandExt;
    let mut cmd = std::process::Command::new("explorer.exe");
    if path.exists() {
        cmd.raw_arg(format!("/select,\"{}\"", path.display()));
    } else if let Some(parent) = path.parent() {
        cmd.arg(parent);
    }
    let _ = cmd.spawn();
}

pub fn set_autostart(on: bool) -> windows::core::Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_SET_VALUE, &mut key).ok()?;
        let result = if on {
            let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            let value: Vec<u16> = format!("\"{exe}\" --background").encode_utf16().chain(Some(0)).collect();
            let bytes = std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2);
            RegSetValueExW(key, RUN_VALUE, None, REG_SZ, Some(bytes)).ok()
        } else {
            let r = RegDeleteValueW(key, RUN_VALUE);
            if r.is_err() && r.0 != 2 { r.ok() } else { Ok(()) }
        };
        let _ = RegCloseKey(key);
        result
    }
}
