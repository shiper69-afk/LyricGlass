use crate::startup;
use crate::wallpaper_select::{choose_wallpaper_parent, DesktopLayout, DesktopWindowInfo};
use parking_lot::Mutex;
use std::sync::LazyLock;

static LAST_ERROR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));
static LAST_DIAGNOSTICS: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub fn last_attach_error() -> Option<String> {
    LAST_ERROR.lock().clone()
}

pub fn last_attach_diagnostics() -> Option<String> {
    LAST_DIAGNOSTICS.lock().clone()
}

pub fn clear_attach_error() {
    *LAST_ERROR.lock() = None;
    *LAST_DIAGNOSTICS.lock() = None;
}

fn record_attach_failure(step: &str, summary: &str, detail: String) {
    startup::step(&format!("wallpaper: {step}: {summary}"));
    startup::log_diagnostic(&format!("WALLPAPER ATTACH FAILED\n{detail}"));
    *LAST_ERROR.lock() = Some(format!("{step}: {summary}"));
    *LAST_DIAGNOSTICS.lock() = Some(detail);
}

#[derive(Debug, Default, Clone)]
pub struct OverlayGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Clone)]
struct WallpaperRestore {
    attached: bool,
    overlay: OverlayGeometry,
    #[cfg(windows)]
    previous_style: isize,
    #[cfg(windows)]
    previous_ex_style: isize,
}

static WALLPAPER_STATE: LazyLock<Mutex<WallpaperRestore>> =
    LazyLock::new(|| Mutex::new(WallpaperRestore::default()));

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    };
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, GetClassNameW, GetParent,
        GetWindowLongPtrW, GetWindowRect, IsWindow, SendMessageTimeoutW, SetParent,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, SMTO_ABORTIFHUNG,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW,
    };

    const PROGMAN_DESKTOP_MSG: u32 = 0x052C;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_POPUP: u32 = 0x8000_0000;
    const WS_CAPTION: u32 = 0x00C0_0000;
    const WS_THICKFRAME: u32 = 0x0004_0000;
    const WS_MINIMIZEBOX: u32 = 0x0002_0000;
    const WS_MAXIMIZEBOX: u32 = 0x0001_0000;
    const WS_SYSMENU: u32 = 0x0008_0000;
    const WS_BORDER: u32 = 0x0080_0000;
    const WS_DLGFRAME: u32 = 0x0040_0000;
    const WS_EX_TOPMOST: u32 = 0x0000_0008;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;
    const WS_EX_NOREDIRECTIONBITMAP: u32 = 0x0020_0000;

    fn hwnd_to_u64(hwnd: HWND) -> u64 {
        hwnd.0 as usize as u64
    }

    fn hwnd_valid(hwnd: HWND) -> bool {
        !hwnd.0.is_null() && unsafe { IsWindow(Some(hwnd)).as_bool() }
    }

    fn progman() -> Option<HWND> {
        let hwnd = unsafe { FindWindowW(w!("Progman"), None).ok()? };
        hwnd_valid(hwnd).then_some(hwnd)
    }

    fn last_error_code() -> u32 {
        unsafe { GetLastError().0 }
    }

    fn windows_build_label() -> String {
        use windows::Win32::System::SystemInformation::GetVersion;
        unsafe {
            let v = GetVersion();
            let build = v >> 16;
            let major = (v >> 8) & 0xff;
            let minor = v & 0xff;
            format!("{major}.{minor} build {build}")
        }
    }

    fn spawn_desktop_workerw(diag: &mut String) -> Result<(), String> {
        let progman = progman().ok_or_else(|| "Progman not found".to_string())?;
        diag.push_str(&format!("progman_hwnd=0x{:x}\n", hwnd_to_u64(progman)));

        let variants: [(isize, isize); 4] = [(0xD, 0), (0xD, 1), (0, 0), (1, 0)];
        for (attempt, (wp, lp)) in variants.iter().enumerate() {
            let mut result = 0usize;
            let r = unsafe {
                SendMessageTimeoutW(
                    progman,
                    PROGMAN_DESKTOP_MSG,
                    WPARAM(*wp as usize),
                    LPARAM(*lp),
                    SMTO_ABORTIFHUNG,
                    1500,
                    Some(&mut result),
                )
            };
            diag.push_str(&format!(
                "spawn attempt {attempt}: msg=0x{PROGMAN_DESKTOP_MSG:x} wParam=0x{wp:x} lParam=0x{lp:x} ok={} result=0x{result:x} lastError={}\n",
                r.0 != 0,
                last_error_code()
            ));
        }
        Ok(())
    }

    struct EnumCollect {
        z: usize,
        windows: Vec<DesktopWindowInfo>,
    }

    unsafe extern "system" fn enum_collect_top(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !hwnd_valid(hwnd) {
            return BOOL::from(true);
        }
        let ctx = &mut *(lparam.0 as *mut EnumCollect);
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class);
        let class_name = if len > 0 {
            String::from_utf16_lossy(&class[..len as usize])
        } else {
            String::new()
        };
        let parent = GetParent(hwnd).unwrap_or(HWND::default());
        let parent_hwnd = if parent.0.is_null() {
            0
        } else {
            hwnd_to_u64(parent)
        };
        let defview =
            FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None).ok();
        let has_defview_child = defview.map(hwnd_valid).unwrap_or(false);
        ctx.windows.push(DesktopWindowInfo {
            hwnd: hwnd_to_u64(hwnd),
            class_name,
            parent_hwnd,
            has_defview_child,
            z_order: ctx.z,
        });
        ctx.z += 1;
        BOOL::from(true)
    }

    fn collect_desktop_windows() -> Vec<DesktopWindowInfo> {
        let mut ctx = EnumCollect {
            z: 0,
            windows: Vec::new(),
        };
        unsafe {
            let _ = EnumWindows(Some(enum_collect_top), LPARAM(&mut ctx as *mut _ as isize));
        }
        ctx.windows
    }

    fn wait_for_workerw(diag: &mut String) -> Result<(HWND, DesktopLayout), String> {
        let progman = progman().ok_or_else(|| "Progman missing after spawn".to_string())?;
        for retry in 0..12 {
            let list = collect_desktop_windows();
            if retry == 0 || retry == 11 {
                for w in &list {
                    if w.class_name == "WorkerW" || w.class_name == "Progman" {
                        diag.push_str(&format!(
                            "  hwnd=0x{:x} class={} parent=0x{:x} defview={} z={}\n",
                            w.hwnd, w.class_name, w.parent_hwnd, w.has_defview_child, w.z_order
                        ));
                    }
                }
            }
            if let Some((host, layout)) =
                choose_wallpaper_parent(&list, hwnd_to_u64(progman))
            {
                let hwnd = HWND(host as *mut core::ffi::c_void);
                if hwnd_valid(hwnd) {
                    diag.push_str(&format!(
                        "workerw retry {retry}: host=0x{host:x} layout={layout:?}\n"
                    ));
                    return Ok((hwnd, layout));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
        Err("WorkerW host not found after retries".into())
    }

    fn parent_chain(hwnd: HWND) -> String {
        let mut parts = Vec::new();
        let mut cur = hwnd;
        for _ in 0..8 {
            if !hwnd_valid(cur) {
                break;
            }
            let mut class = [0u16; 64];
            let len = unsafe { GetClassNameW(cur, &mut class) };
            let name = if len > 0 {
                String::from_utf16_lossy(&class[..len as usize])
            } else {
                "?".into()
            };
            parts.push(format!("0x{:x}({name})", hwnd_to_u64(cur)));
            let parent = unsafe { GetParent(cur) }.unwrap_or(HWND::default());
            if parent.0.is_null() || parent == cur {
                break;
            }
            cur = parent;
        }
        parts.join(" -> ")
    }

    fn monitor_rect_for_window(hwnd: HWND) -> (RECT, u32) {
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let dpi = if dpi == 0 { 96 } else { dpi };
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let rect = if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            info.rcMonitor
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        };
        (rect, dpi)
    }

    fn child_screen_rect(parent: HWND, hwnd: HWND) -> (i32, i32, i32, i32) {
        let (monitor, dpi) = monitor_rect_for_window(hwnd);
        let width = monitor.right - monitor.left;
        let height = monitor.bottom - monitor.top;
        let parent_rect = unsafe {
            let mut r = RECT::default();
            let _ = GetWindowRect(parent, &mut r);
            r
        };
        // Position relative to parent client (WorkerW origin is screen coords).
        let x = monitor.left - parent_rect.left;
        let y = monitor.top - parent_rect.top;
        let _ = dpi;
        (x, y, width, height)
    }

    fn rollback_attach(hwnd: HWND, restore: &WallpaperRestore) {
        unsafe {
            let _ = SetParent(hwnd, None);
            SetWindowLongPtrW(hwnd, GWL_STYLE, restore.previous_style);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, restore.previous_ex_style);
            let geo = &restore.overlay;
            let _ = SetWindowPos(
                hwnd,
                None,
                geo.x,
                geo.y,
                geo.width as i32,
                geo.height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }

    pub fn attach_wallpaper(hwnd: HWND) -> Result<(), String> {
        clear_attach_error();
        let mut diag = format!(
            "windows: {}\nstep: begin attach\nour_hwnd=0x{:x}\n",
            windows_build_label(),
            hwnd_to_u64(hwnd)
        );

        let mut guard = WALLPAPER_STATE.lock();
        if guard.attached {
            return Ok(());
        }

        if let Err(e) = spawn_desktop_workerw(&mut diag) {
            record_attach_failure("spawn", &e, diag);
            return Err(e);
        }

        let (workerw, layout) = match wait_for_workerw(&mut diag) {
            Ok(v) => v,
            Err(e) => {
                record_attach_failure("find_workerw", &e, diag);
                return Err(e);
            }
        };

        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            guard.overlay = OverlayGeometry {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(0) as u32,
                height: (rect.bottom - rect.top).max(0) as u32,
            };
        }
        guard.previous_style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        guard.previous_ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        let snapshot = guard.clone();
        drop(guard);

        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }

        let attach_result: Result<(), String> = (|| {
            unsafe {
                SetParent(hwnd, Some(workerw)).map_err(|e| {
                    format!("SetParent -> {e} (lastError={})", last_error_code())
                })?;

                let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                let strip = WS_POPUP
                    | WS_CAPTION
                    | WS_THICKFRAME
                    | WS_MINIMIZEBOX
                    | WS_MAXIMIZEBOX
                    | WS_SYSMENU
                    | WS_BORDER
                    | WS_DLGFRAME;
                let style = (style & !strip) | WS_CHILD;
                SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

                let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                ex &= !WS_EX_TOPMOST;
                ex &= !WS_EX_APPWINDOW;
                ex |= WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP;
                // Keep WS_EX_LAYERED when already set (Tauri transparency).
                if (snapshot.previous_ex_style as u32) & WS_EX_LAYERED != 0 {
                    ex |= WS_EX_LAYERED;
                }
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);

                let (x, y, w, h) = child_screen_rect(workerw, hwnd);
                let (_, dpi) = monitor_rect_for_window(hwnd);
                diag.push_str(&format!(
                    "layout={layout:?} parent=0x{:x} dpi={dpi}\nsetpos child x={x} y={y} w={w} h={h}\n",
                    hwnd_to_u64(workerw)
                ));

                SetWindowPos(
                    hwnd,
                    None,
                    x,
                    y,
                    w,
                    h,
                    SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                )
                .map_err(|e| {
                    format!("SetWindowPos -> {e} (lastError={})", last_error_code())
                })?;

                let parent = GetParent(hwnd).unwrap_or(HWND::default());
                if parent != workerw {
                    return Err(format!(
                        "verify: parent=0x{:x} expected WorkerW 0x{:x}",
                        hwnd_to_u64(parent),
                        hwnd_to_u64(workerw)
                    ));
                }

                diag.push_str(&format!("parent_chain: {}\n", parent_chain(hwnd)));
                Ok(())
            }
        })();

        let mut guard = WALLPAPER_STATE.lock();
        if let Err(ref e) = attach_result {
            rollback_attach(hwnd, &snapshot);
            record_attach_failure("attach", e, diag);
            return Err(e.clone());
        }

        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        guard.attached = true;
        startup::step(&format!("wallpaper: attached ({layout:?})"));
        diag.push_str("step: attach OK\n");
        *LAST_DIAGNOSTICS.lock() = Some(diag);
        Ok(())
    }

    pub fn detach_wallpaper(hwnd: HWND) -> Result<(), String> {
        let mut guard = WALLPAPER_STATE.lock();
        if !guard.attached {
            return Ok(());
        }

        unsafe {
            SetParent(hwnd, None).map_err(|e| format!("SetParent(desktop) failed: {e}"))?;
            SetWindowLongPtrW(hwnd, GWL_STYLE, guard.previous_style);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, guard.previous_ex_style);

            let geo = guard.overlay.clone();
            SetWindowPos(
                hwnd,
                None,
                geo.x,
                geo.y,
                geo.width as i32,
                geo.height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .map_err(|e| format!("SetWindowPos (restore) failed: {e}"))?;
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        guard.attached = false;
        clear_attach_error();
        startup::step("wallpaper: detached, overlay geometry restored");
        Ok(())
    }

    pub fn take_overlay_geometry() -> Option<OverlayGeometry> {
        let guard = WALLPAPER_STATE.lock();
        if guard.overlay.width > 0 && guard.overlay.height > 0 {
            Some(guard.overlay.clone())
        } else {
            None
        }
    }

    pub fn is_attached() -> bool {
        WALLPAPER_STATE.lock().attached
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn attach_wallpaper(_hwnd: ()) -> Result<(), String> {
        Err("Wallpaper attach is Windows-only".into())
    }

    pub fn detach_wallpaper(_hwnd: ()) -> Result<(), String> {
        Ok(())
    }

    pub fn take_overlay_geometry() -> Option<OverlayGeometry> {
        None
    }

    pub fn is_attached() -> bool {
        false
    }
}

#[cfg(windows)]
pub use imp::{attach_wallpaper, detach_wallpaper, is_attached, take_overlay_geometry};

#[cfg(not(windows))]
pub use imp::{attach_wallpaper, detach_wallpaper, is_attached, take_overlay_geometry};

#[cfg(test)]
mod tests {
    use super::OverlayGeometry;

    #[test]
    fn overlay_geometry_default_is_zero() {
        let g = OverlayGeometry::default();
        assert_eq!(g.width, 0);
        assert_eq!(g.height, 0);
    }
}