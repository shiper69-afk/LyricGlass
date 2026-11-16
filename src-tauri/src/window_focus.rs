//! Restore and focus the main overlay from single-instance, tray, or close-to-tray flows.

#[cfg(windows)]
mod imp {
    use crate::app_version::{
        compare_version_strings, incoming_is_newer, normalize_version_string,
    };
    use crate::startup;
    use crate::{detach_wallpaper_for_ui_focus, apply_topmost, SharedFlags};
    use std::cmp::Ordering;
    use std::path::Path;
    use tauri::{AppHandle, Manager, WebviewWindowBuilder};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    };
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, BringWindowToTop, GetWindowPlacement, GetWindowRect, IsIconic,
        IsWindowVisible, SetForegroundWindow, SetWindowPlacement, ShowWindow, SW_RESTORE, SW_SHOW,
        WINDOWPLACEMENT,
    };

    use crate::wallpaper_placement::{
        clamp_rect_to_work_area, placement_coords_sane, restore_main_placement,
        save_main_placement, RectI32,
    };

    fn wide(path: &str) -> Vec<u16> {
        use std::os::windows::prelude::OsStrExt;
        std::path::Path::new(path)
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

  pub fn read_pe_file_version(exe_path: &str) -> Option<String> {
        let path = wide(exe_path);
        unsafe {
            let size = GetFileVersionInfoSizeW(PCWSTR(path.as_ptr()), None);
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            GetFileVersionInfoW(
                PCWSTR(path.as_ptr()),
                Some(0),
                size,
                data.as_mut_ptr() as _,
            )
            .ok()?;
            let mut value_ptr = std::ptr::null_mut();
            let mut value_len = 0u32;
            let subblock = wide(r"\StringFileInfo\040904b0\FileVersion");
            VerQueryValueW(
                data.as_ptr() as _,
                PCWSTR(subblock.as_ptr()),
                &mut value_ptr,
                &mut value_len,
            )
            .ok()?;
            if value_ptr.is_null() || value_len == 0 {
                let subblock = wide(r"\StringFileInfo\040904e4\FileVersion");
                VerQueryValueW(
                    data.as_ptr() as _,
                    PCWSTR(subblock.as_ptr()),
                    &mut value_ptr,
                    &mut value_len,
                )
                .ok()?;
            }
            if value_ptr.is_null() {
                return None;
            }
            let utf16 = std::slice::from_raw_parts(value_ptr as *const u16, value_len as usize);
            let s = String::from_utf16_lossy(utf16);
            let trimmed = s.trim_matches('\0').trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(normalize_version_string(&trimmed))
            }
        }
    }

    unsafe fn primary_work_area() -> RectI32 {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mon = MonitorFromWindow(HWND(std::ptr::null_mut()), MONITOR_DEFAULTTOPRIMARY);
        let _ = GetMonitorInfoW(mon, &mut info);
        RectI32 {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        }
    }

    unsafe fn log_hwnd(tag: &str, hwnd: HWND) {
        let visible = IsWindowVisible(hwnd).as_bool();
        let iconic = IsIconic(hwnd).as_bool();
        let mut rc = windows::Win32::Foundation::RECT::default();
        let _ = GetWindowRect(hwnd, &mut rc);
        startup::step(&format!(
            "focus [{tag}]: IsWindowVisible={visible} IsIconic={iconic} pos=({},{})-({},{})",
            rc.left, rc.top, rc.right, rc.bottom
        ));
    }

    unsafe fn force_foreground(hwnd: HWND) {
        let pid = GetCurrentProcessId();
        let _ = AllowSetForegroundWindow(pid);
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }

    unsafe fn fix_placement_on_screen(hwnd: HWND, tag: &str) {
        let work = primary_work_area();
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        if GetWindowPlacement(hwnd, &mut placement).is_err() {
            startup::step(&format!("focus [{tag}]: GetWindowPlacement failed"));
            return;
        }
        let rc = placement.rcNormalPosition;
        let mut rect = RectI32 {
            left: rc.left,
            top: rc.top,
            right: rc.right,
            bottom: rc.bottom,
        };
        let needs_fix = !placement_coords_sane(rc.left, rc.top)
            || !placement_coords_sane(rc.right, rc.bottom);
        if needs_fix {
            startup::step(&format!(
                "focus [{tag}]: off-screen placement ({},{}) — clamping to work area",
                rc.left, rc.top
            ));
            clamp_rect_to_work_area(&mut rect, &work);
            placement.rcNormalPosition.left = rect.left;
            placement.rcNormalPosition.top = rect.top;
            placement.rcNormalPosition.right = rect.right;
            placement.rcNormalPosition.bottom = rect.bottom;
            placement.showCmd = 1; // SW_SHOWNORMAL
            let _ = SetWindowPlacement(hwnd, &placement);
        }
    }

    fn ensure_main_window(app: &AppHandle) -> Option<tauri::WebviewWindow> {
        if let Some(w) = app.get_webview_window("main") {
            return Some(w);
        }
        startup::step("focus: main webview missing — recreating from config");
        let Some(main_cfg) = app
            .config()
            .app
            .windows
            .iter()
            .find(|w| w.label == "main")
        else {
            startup::step("focus: main window config missing");
            return None;
        };
        match WebviewWindowBuilder::from_config(app, main_cfg) {
            Ok(builder) => match builder.build() {
                Ok(w) => {
                    let flags = app.state::<SharedFlags>();
                    apply_topmost(&w, &flags.lock());
                    startup::step("focus: main webview recreated");
                    Some(w)
                }
                Err(e) => {
                    startup::step(&format!("focus: recreate build failed: {e}"));
                    None
                }
            },
            Err(e) => {
                startup::step(&format!("focus: recreate from_config failed: {e}"));
                None
            }
        }
    }

    pub fn reveal_main_window(app: &AppHandle, trigger: &str) {
        startup::step(&format!("focus [{trigger}]: begin"));
        detach_wallpaper_for_ui_focus(app);
        let Some(window) = ensure_main_window(app) else {
            startup::step(&format!("focus [{trigger}]: aborted — no main window"));
            return;
        };
        let Ok(hwnd_raw) = window.hwnd() else {
            startup::step(&format!("focus [{trigger}]: hwnd unavailable"));
            return;
        };
        let hwnd = HWND(hwnd_raw.0 as _);
        unsafe {
            log_hwnd(&format!("{trigger}/before"), hwnd);
            fix_placement_on_screen(hwnd, trigger);
            let saved = save_main_placement(hwnd);
            let _ = restore_main_placement(hwnd, &saved);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        unsafe {
            force_foreground(hwnd);
            log_hwnd(&format!("{trigger}/after"), hwnd);
        }
        startup::step(&format!("focus [{trigger}]: complete"));
    }

    fn version_from_argv0(args: &[String]) -> Option<String> {
        let exe = args.first()?;
        if Path::new(exe).exists() {
            read_pe_file_version(exe)
        } else {
            None
        }
    }

    fn request_version_handoff(app: &AppHandle, exe_path: &str, incoming: &str, running: &str) {
        startup::step(&format!(
            "single-instance: handoff — incoming {incoming} > running {running}, exiting for respawn"
        ));
        startup::step("single-instance handoff: detach wallpaper if needed");
        detach_wallpaper_for_ui_focus(app);
        startup::step("single-instance handoff: releasing instance lock");
        tauri_plugin_single_instance::destroy(app);
        match std::process::Command::new(exe_path).spawn() {
            Ok(child) => startup::step(&format!(
                "single-instance handoff: respawned pid={}",
                child.id()
            )),
            Err(e) => startup::step(&format!("single-instance handoff: respawn failed: {e}")),
        }
        app.exit(0);
    }

    pub fn on_second_instance(app: &AppHandle, args: Vec<String>, cwd: String) {
        let running = env!("CARGO_PKG_VERSION");
        let incoming_ver = version_from_argv0(&args);
        startup::step(&format!(
            "single-instance: second launch cwd={cwd} argv={args:?} incoming_ver={incoming_ver:?} running={running}"
        ));
        let decision = match incoming_ver.as_deref() {
            Some(incoming) if incoming_is_newer(incoming, running) => "upgrade-handoff",
            Some(incoming) => {
                let ord = compare_version_strings(incoming, running);
                if ord == Ordering::Less {
                    "focus-existing-newer-running"
                } else {
                    "focus-existing"
                }
            }
            None => "focus-existing-no-incoming-version",
        };
        startup::step(&format!("single-instance: decision={decision}"));
        let exe = args.first().cloned();
        let incoming = incoming_ver.clone();
        let decision = decision.to_string();
        let app_main = app.clone();
        let _ = app.clone().run_on_main_thread(move || {
            if decision == "upgrade-handoff" {
                if let Some(exe) = exe {
                    let incoming = incoming.as_deref().unwrap_or("?");
                    request_version_handoff(&app_main, &exe, incoming, running);
                    return;
                }
            }
            reveal_main_window(&app_main, "single-instance");
        });
    }
}

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
mod imp {
    use crate::app_version::incoming_is_newer;
    use crate::startup;
    use tauri::AppHandle;

    pub fn read_pe_file_version(_: &str) -> Option<String> {
        None
    }

    pub fn reveal_main_window(app: &AppHandle, trigger: &str) {
        startup::step(&format!("focus [{trigger}]: begin (stub)"));
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        }
    }

    pub fn on_second_instance(app: &AppHandle, args: Vec<String>, cwd: String) {
        let running = env!("CARGO_PKG_VERSION");
        let incoming = args.get(1).map(|s| s.as_str());
        if let Some(inc) = incoming {
            if incoming_is_newer(inc, running) {
                startup::step("single-instance: upgrade handoff not supported on this platform");
                return;
            }
        }
        startup::step(&format!("single-instance: second launch cwd={cwd}"));
        reveal_main_window(app, "single-instance");
    }
}

#[cfg(not(windows))]
pub use imp::*;