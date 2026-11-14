use parking_lot::Mutex;
use std::sync::LazyLock;

#[derive(Debug, Default)]
struct WallpaperRestore {
    attached: bool,
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
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, FindWindowW, GetWindowLongPtrW, SendMessageW, SetParent,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, WM_USER,
    };

    fn hwnd_valid(hwnd: HWND) -> bool {
        !hwnd.0.is_null()
    }

    fn find_wallpaper_workerw() -> Option<HWND> {
        let progman = unsafe { FindWindowW(w!("Progman"), None).ok()? };
        if !hwnd_valid(progman) {
            return None;
        }

        unsafe {
            let _ = SendMessageW(progman, WM_USER + 0x052C, None, None);
        }

        let defview = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok()?;
        if !hwnd_valid(defview) {
            return None;
        }

        let workerw = FindWindowExW(None, Some(defview), w!("WorkerW"), None).ok()?;
        if hwnd_valid(workerw) {
            Some(workerw)
        } else {
            None
        }
    }

    pub fn attach_wallpaper(hwnd: HWND) -> Result<(), String> {
        let workerw = find_wallpaper_workerw().ok_or("WorkerW not found (is Explorer running?)")?;

        let mut guard = WALLPAPER_STATE.lock();
        if guard.attached {
            return Ok(());
        }

        unsafe {
            guard.previous_style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            guard.previous_ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

            SetParent(hwnd, Some(workerw)).map_err(|e| format!("SetParent failed: {e}"))?;

            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let style = (style as u64 & !0x00C00000) | 0x10000000;
            SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);

            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE
                    | SWP_SHOWWINDOW,
            )
            .map_err(|e| format!("SetWindowPos failed: {e}"))?;
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        guard.attached = true;
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
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        guard.attached = false;
        Ok(())
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
        Err("Wallpaper attach is Windows-only".into())
    }

    pub fn is_attached() -> bool {
        false
    }
}

#[cfg(windows)]
pub use imp::{attach_wallpaper, detach_wallpaper, is_attached};

#[cfg(not(windows))]
pub use imp::{attach_wallpaper, detach_wallpaper, is_attached};