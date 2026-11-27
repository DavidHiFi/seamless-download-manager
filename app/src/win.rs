//! The small bits of Windows the window needs: glass, single instance, opening files, start with Windows.

use std::ffi::c_void;
use std::path::Path;

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_BLURBEHIND, DWMWINDOWATTRIBUTE, DwmEnableBlurBehindWindow, DwmExtendFrameIntoClientArea,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, HGDIOBJ};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, IsIconic, RegisterWindowMessageW, SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{BOOL, HSTRING, PCWSTR, s, w};

const DWMWA_USE_IMMERSIVE_DARK_MODE: i32 = 20;
const DWMWA_WINDOW_CORNER_PREFERENCE: i32 = 33;
const DWMWA_CAPTION_COLOR: i32 = 35;
const DWMWA_SYSTEMBACKDROP_TYPE: i32 = 38;
const WCA_ACCENT_POLICY: i32 = 19;
const ACCENT_DISABLED: i32 = 0;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: i32 = 4;
const ACCENT_FLAGS: u32 = 2;
const ACCENT_COLOR: u32 = 0x012E1E1E;
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: PCWSTR = w!("SeamlessDownloadManager");

#[repr(C)]
struct AccentPolicy {
    state: i32,
    flags: u32,
    gradient_color: u32,
    animation_id: u32,
}

#[repr(C)]
struct WindowCompositionAttributeData {
    attribute: i32,
    data: *mut c_void,
    size: usize,
}

type SetWindowCompositionAttributeFn =
    unsafe extern "system" fn(HWND, *mut WindowCompositionAttributeData) -> BOOL;

unsafe fn set_window_composition_attribute(hwnd: HWND, policy: &AccentPolicy) -> bool {
    static PROC: std::sync::OnceLock<Option<SetWindowCompositionAttributeFn>> =
        std::sync::OnceLock::new();
    let proc = PROC.get_or_init(|| unsafe {
        let module = GetModuleHandleW(w!("user32.dll")).ok()?;
        let proc = GetProcAddress(module, s!("SetWindowCompositionAttribute"))?;
        Some(std::mem::transmute::<
            unsafe extern "system" fn() -> isize,
            SetWindowCompositionAttributeFn,
        >(proc))
    });
    let Some(proc) = proc else { return false };

    let mut data = WindowCompositionAttributeData {
        attribute: WCA_ACCENT_POLICY,
        data: policy as *const AccentPolicy as *mut c_void,
        size: std::mem::size_of::<AccentPolicy>(),
    };
    unsafe { proc(hwnd, &mut data).as_bool() }
}

pub fn hwnd_of(window: &slint::Window) -> Option<HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle();
    let raw = handle.window_handle().ok()?.as_raw();
    match raw {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut _)),
        _ => None,
    }
}

/// Applies the house's dark acrylic material to the native window.
pub fn apply_glass(hwnd: HWND) {
    unsafe {
        let dark = 1i32;
        let no_backdrop = 1i32;
        let rounded = 2i32;
        let no_caption_color = 0xFFFFFFFEu32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_USE_IMMERSIVE_DARK_MODE),
            &dark as *const i32 as *const c_void,
            std::mem::size_of_val(&dark) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_SYSTEMBACKDROP_TYPE),
            &no_backdrop as *const i32 as *const c_void,
            std::mem::size_of_val(&no_backdrop) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_WINDOW_CORNER_PREFERENCE),
            &rounded as *const i32 as *const c_void,
            std::mem::size_of_val(&rounded) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_CAPTION_COLOR),
            &no_caption_color as *const u32 as *const c_void,
            std::mem::size_of_val(&no_caption_color) as u32,
        );

        let disabled = AccentPolicy {
            state: ACCENT_DISABLED,
            flags: 0,
            gradient_color: 0,
            animation_id: 0,
        };
        let _ = set_window_composition_attribute(hwnd, &disabled);

        let region = CreateRectRgn(0, 0, -1, -1);
        if !region.0.is_null() {
            let blur = DWM_BLURBEHIND {
                dwFlags: 0x1,
                fEnable: BOOL(1),
                hRgnBlur: region,
                fTransitionOnMaximized: BOOL(0),
            };
            let _ = DwmEnableBlurBehindWindow(hwnd, &blur);
            let _ = DeleteObject(HGDIOBJ(region.0));
        }

        let acrylic = AccentPolicy {
            state: ACCENT_ENABLE_ACRYLICBLURBEHIND,
            flags: ACCENT_FLAGS,
            gradient_color: ACCENT_COLOR,
            animation_id: 0,
        };
        let _ = set_window_composition_attribute(hwnd, &acrylic);

        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
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
