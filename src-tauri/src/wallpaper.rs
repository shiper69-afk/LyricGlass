use crate::startup;
use parking_lot::Mutex;
use std::sync::LazyLock;

static LAST_ERROR: LazyLock<Mutex<Option<String>>> = LazyLock::new(|| Mutex::new(None));

pub fn last_attach_error() -> Option<String> {
    LAST_ERROR.lock().clone()
}

pub fn clear_attach_error() {
    *LAST_ERROR.lock() = None;
}

fn record_error(msg: String) {
    startup::step(&format!("wallpaper: {msg}"));
    *LAST_ERROR.lock() = Some(msg);
}

#[derive(Debug, Default, Clone)]
pub struct OverlayGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default)]
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
    use windows::core::w;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, GetClassNameW, GetMonitorInfoW, GetParent,
        GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, IsWindow, MonitorFromWindow,
        SendMessageW, SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE,
        GWL_STYLE, MONITORINFO, MONITOR_DEFAULTTONEAREST, SM_CXSCREEN, SM_CYSCREEN,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, WM_USER,
    };

    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

    fn hwnd_valid(hwnd: HWND) -> bool {
        !hwnd.0.is_null() && unsafe { IsWindow(hwnd).as_bool() }
    }

    fn progman() -> Option<HWND> {
        let hwnd = unsafe { FindWindowW(w!("Progman"), None).ok()? };
        hwnd_valid(hwnd).then_some(hwnd)
    }

    fn spawn_desktop_workerw() {
        if let Some(progman) = progman() {
            unsafe {
                let _ = SendMessageW(progman, WM_USER + 0x052C, None, None);
            }
        }
    }

    /// WorkerW under Progman that does **not** host `SHELLDLL_DefView` (Win11 24H2+ layout).
    fn workerw_under_progman(progman: HWND) -> Option<HWND> {
        let mut sibling = HWND::default();
        loop {
            let next = unsafe {
                FindWindowExW(Some(progman), Some(sibling), w!("WorkerW"), None).ok()?
            };
            if !hwnd_valid(next) {
                break;
            }
            let defview =
                unsafe { FindWindowExW(Some(next), None, w!("SHELLDLL_DefView"), None).ok() };
            let has_defview = defview.map(hwnd_valid).unwrap_or(false);
            if !has_defview {
                return Some(next);
            }
            sibling = next;
        }
        None
    }

    /// Legacy: WorkerW immediately after the icon `SHELLDLL_DefView` sibling.
    fn workerw_legacy(progman: HWND) -> Option<HWND> {
        let defview = unsafe {
            FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok()?
        };
        if !hwnd_valid(defview) {
            return None;
        }
        let workerw = unsafe { FindWindowExW(None, Some(defview), w!("WorkerW"), None).ok()? };
        hwnd_valid(workerw).then_some(workerw)
    }

  struct EnumCtx {
      progman: HWND,
      found: HWND,
  }

    unsafe extern "system" fn enum_workerw(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut EnumCtx);
        if !hwnd_valid(hwnd) {
            return BOOL::from(true);
        }
        let mut class = [0u16; 32];
        let len = GetClassNameW(hwnd, &mut class);
        if len == 0 {
            return BOOL::from(true);
        }
        let name = String::from_utf16_lossy(&class[..len as usize]);
        if name != "WorkerW" {
            return BOOL::from(true);
        }
        let parent = GetParent(hwnd).unwrap_or(HWND::default());
        if parent == ctx.progman {
            let defview =
                FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None).ok();
            let has_defview = defview.map(hwnd_valid).unwrap_or(false);
            if !has_defview && ctx.found.0.is_null() {
                ctx.found = hwnd;
            }
        }
        BOOL::from(true)
    }

    fn find_wallpaper_workerw() -> Option<HWND> {
        spawn_desktop_workerw();
        let progman = progman()?;

        if let Some(w) = workerw_under_progman(progman) {
            return Some(w);
        }
        if let Some(w) = workerw_legacy(progman) {
            return Some(w);
        }

        let mut ctx = EnumCtx {
            progman,
            found: HWND::default(),
        };
        unsafe {
            let _ = EnumWindows(Some(enum_workerw), LPARAM(&mut ctx as *mut _ as isize));
        }
        hwnd_valid(ctx.found).then_some(ctx.found)
    }

    fn primary_screen_rect() -> RECT {
        let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        RECT {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        }
    }

    fn monitor_rect(hwnd: HWND) -> RECT {
        unsafe {
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(mon, &mut info).as_bool() {
                info.rcMonitor
            } else {
                primary_screen_rect()
            }
        }
    }

    pub fn attach_wallpaper(hwnd: HWND) -> Result<(), String> {
        clear_attach_error();
        let workerw =
            find_wallpaper_workerw().ok_or_else(|| {
                let msg = "WorkerW not found (Explorer desktop layer unavailable)".to_string();
                record_error(msg.clone());
                msg
            })?;

        let mut guard = WALLPAPER_STATE.lock();
        if guard.attached {
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

            guard.previous_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            guard.previous_ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

            SetParent(hwnd, Some(workerw)).map_err(|e| {
                let msg = format!("SetParent failed: {e}");
                record_error(msg.clone());
                msg
            })?;

            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let style = (style as u64 & !0x00C0_0000) | 0x1000_0000;
            SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let ex = ex | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);

            let area = monitor_rect(workerw);
            let width = area.right - area.left;
            let height = area.bottom - area.top;
            SetWindowPos(
                hwnd,
                None,
                area.left,
                area.top,
                width,
                height,
                SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .map_err(|e| {
                let _ = SetParent(hwnd, None);
                let msg = format!("SetWindowPos (fullscreen) failed: {e}");
                record_error(msg.clone());
                msg
            })?;
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        guard.attached = true;
        startup::step("wallpaper: attached to WorkerW");
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