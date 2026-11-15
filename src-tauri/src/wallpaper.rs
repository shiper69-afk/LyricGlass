use crate::startup;
use crate::wallpaper_log;
use crate::wallpaper_select::{choose_wallpaper_parent, DesktopLayout, DesktopWindowInfo};
use parking_lot::Mutex;
use std::sync::LazyLock;

pub fn last_attach_error() -> Option<String> {
    wallpaper_log::last_attempt().and_then(|a| a.error)
}

pub fn last_attach_diagnostics() -> Option<String> {
    wallpaper_log::last_attempt().map(|a| a.trace)
}

fn finish_attempt(
    trigger: &str,
    outcome: &str,
    attached_after: bool,
    error: Option<String>,
    trace: String,
) {
    startup::step(&format!("wallpaper: {outcome} trigger={trigger} attached={attached_after}"));
    wallpaper_log::record_attempt(trigger, outcome, attached_after, error.clone(), trace.clone());
    if error.is_some() {
        startup::log_diagnostic(&format!("WALLPAPER ATTACH\n{trace}"));
    }
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
        HWND_BOTTOM, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW,
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

    #[repr(C)]
    struct RtlOsVersionInfo {
        dw_os_version_info_size: u32,
        dw_major_version: u32,
        dw_minor_version: u32,
        dw_build_number: u32,
        dw_platform_id: u32,
        sz_csd_version: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut RtlOsVersionInfo) -> i32;
    }

    fn windows_build_label() -> String {
        unsafe {
            let mut v = RtlOsVersionInfo {
                dw_os_version_info_size: std::mem::size_of::<RtlOsVersionInfo>() as u32,
                dw_major_version: 0,
                dw_minor_version: 0,
                dw_build_number: 0,
                dw_platform_id: 0,
                sz_csd_version: [0; 128],
            };
            let status = RtlGetVersion(&mut v);
            format!(
                "RtlGetVersion status={status} {}.{}.{} build {}",
                v.dw_major_version,
                v.dw_minor_version,
                v.dw_build_number,
                v.dw_build_number
            )
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

    fn log_shell_defview(diag: &mut String) {
        let dv = unsafe { FindWindowExW(None, None, w!("SHELLDLL_DefView"), None).ok() };
        match dv {
            Some(hwnd) if hwnd_valid(hwnd) => {
                let parent = unsafe { GetParent(hwnd) }.unwrap_or(HWND::default());
                diag.push_str(&format!(
                    "SHELLDLL_DefView hwnd=0x{:x} parent=0x{:x} parent_chain={}\n",
                    hwnd_to_u64(hwnd),
                    hwnd_to_u64(parent),
                    parent_chain(hwnd)
                ));
            }
            _ => diag.push_str("SHELLDLL_DefView: not found\n"),
        }
    }

    fn apply_wallpaper_styles(hwnd: HWND, snapshot: &WallpaperRestore, diag: &mut String) -> Result<(), String> {
        unsafe {
            let style_before = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let ex_before = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            diag.push_str(&format!(
                "style_before=0x{style_before:x} ex_before=0x{ex_before:x}\n"
            ));

            let style = style_before as u32;
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

            let mut ex = ex_before as u32;
            ex &= !WS_EX_TOPMOST;
            ex &= !WS_EX_APPWINDOW;
            ex |= WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP;
            if (snapshot.previous_ex_style as u32) & WS_EX_LAYERED != 0 {
                ex |= WS_EX_LAYERED;
            }
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);

            let style_after = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let ex_after = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            diag.push_str(&format!(
                "style_after=0x{style_after:x} ex_after=0x{ex_after:x}\n"
            ));
        }
        Ok(())
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

    fn reparent_fullscreen(
        hwnd: HWND,
        parent: HWND,
        host_label: &str,
        layout: Option<DesktopLayout>,
        snapshot: &WallpaperRestore,
        diag: &mut String,
        insert_bottom: bool,
    ) -> Result<(), String> {
        unsafe {
            let sp = SetParent(hwnd, Some(parent));
            diag.push_str(&format!(
                "SetParent({host_label}) ret={sp:?} lastError={}\n",
                last_error_code()
            ));
            sp.map_err(|e| format!("SetParent({host_label}): {e}"))?;

            apply_wallpaper_styles(hwnd, snapshot, diag)?;

            let (x, y, w, h) = child_screen_rect(parent, hwnd);
            let (_, dpi) = monitor_rect_for_window(hwnd);
            diag.push_str(&format!(
                "host={host_label} layout={layout:?} parent_hwnd=0x{:x} dpi={dpi}\nsetpos x={x} y={y} w={w} h={h} z_bottom={insert_bottom}\n",
                hwnd_to_u64(parent)
            ));

            let z = if insert_bottom { Some(HWND_BOTTOM) } else { None };
            SetWindowPos(
                hwnd,
                z,
                x,
                y,
                w,
                h,
                SWP_FRAMECHANGED | SWP_NOACTIVATE,
            )
            .map_err(|e| format!("SetWindowPos({host_label}): {e} (lastError={})", last_error_code()))?;

            let got_parent = GetParent(hwnd).unwrap_or(HWND::default());
            diag.push_str(&format!(
                "verify GetParent=0x{:x} expected=0x{:x}\n",
                hwnd_to_u64(got_parent),
                hwnd_to_u64(parent)
            ));
            if got_parent != parent {
                return Err(format!(
                    "parent verify failed: got 0x{:x} want 0x{:x}",
                    hwnd_to_u64(got_parent),
                    hwnd_to_u64(parent)
                ));
            }
            diag.push_str(&format!("our_parent_chain: {}\n", parent_chain(hwnd)));
        }
        Ok(())
    }

    pub fn attach_wallpaper(hwnd: HWND, trigger: &str) -> Result<(), String> {
        let mut diag = format!(
            "windows: {}\ntrigger: {trigger}\nstep: begin attach\nour_hwnd=0x{:x}\n",
            windows_build_label(),
            hwnd_to_u64(hwnd)
        );
        log_shell_defview(&mut diag);

        let mut guard = WALLPAPER_STATE.lock();
        if guard.attached {
            finish_attempt(trigger, "already_attached", true, None, diag);
            return Ok(());
        }

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

        if let Err(e) = spawn_desktop_workerw(&mut diag) {
            finish_attempt(trigger, "failure", false, Some(format!("spawn: {e}")), diag);
            return Err(e);
        }

        let host_result = wait_for_workerw(&mut diag);
        if let Err(ref e) = host_result {
            diag.push_str(&format!("find_workerw failed: {e}\n"));
        }
        let progman_hwnd = progman();

        let mut attach_result: Result<(), String> = Err("no host".into());

        if let Ok((workerw, layout)) = host_result {
            diag.push_str(&format!("chosen_host=WorkerW 0x{:x}\n", hwnd_to_u64(workerw)));
            attach_result =
                reparent_fullscreen(hwnd, workerw, "WorkerW", Some(layout), &snapshot, &mut diag, false);
        }

        if attach_result.is_err() {
            if let Some(progman) = progman_hwnd {
                diag.push_str("fallback: Progman direct child (z-order bottom)\n");
                attach_result = reparent_fullscreen(
                    hwnd,
                    progman,
                    "Progman",
                    None,
                    &snapshot,
                    &mut diag,
                    true,
                );
            } else {
                diag.push_str("fallback: Progman not found\n");
            }
        }

        let mut guard = WALLPAPER_STATE.lock();
        match attach_result {
            Ok(()) => {
                guard.attached = true;
                diag.push_str("outcome: success\n");
                finish_attempt(trigger, "success", true, None, diag);
                Ok(())
            }
            Err(ref e) => {
                rollback_attach(hwnd, &snapshot);
                diag.push_str(&format!("outcome: failure ({e})\n"));
                finish_attempt(trigger, "failure", false, Some(e.clone()), diag);
                Err(e.clone())
            }
        }
    }

    pub fn detach_wallpaper(hwnd: HWND, trigger: &str) -> Result<(), String> {
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
        let trace = format!("detach trigger={trigger} restored_geometry={:?}", guard.overlay);
        finish_attempt(trigger, "detached", false, None, trace);
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

    pub fn attach_wallpaper(_hwnd: (), _trigger: &str) -> Result<(), String> {
        Err("Wallpaper attach is Windows-only".into())
    }

    pub fn detach_wallpaper(_hwnd: (), _trigger: &str) -> Result<(), String> {
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