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
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, FindWindowW, GetWindowLongPtrW, SendMessageW, SetParent,
        SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, WM_USER,
    };

    fn find_wallpaper_workerw() -> Option<HWND> {
        let progman = unsafe { FindWindowW(w!("Progman"), None) };
        if progman.0 == 0 {
            return None;
        }

        unsafe {
            let _ = SendMessageW(progman, WM_USER + 0x052C, None, None);
        }

        let mut workerw = HWND::default();
        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let shell = FindWindowExW(hwnd, HWND::default(), w!("SHELLDLL_DefView"), None);
            if shell.0 != 0 {
                let behind = FindWindowExW(HWND::default(), hwnd, w!("WorkerW"), None);
                if behind.0 != 0 {
                    let target = lparam.0 as *mut HWND;
                    unsafe {
                        *target = behind;
                    }
                }
            }
            TRUE
        }

        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut workerw as *mut _ as isize));
        }

        if workerw.0 != 0 {
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

            SetParent(hwnd, workerw)
                .map_err(|e| format!("SetParent failed: {e}"))?;
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let style = (style as u64 & !0x00C00000) | 0x10000000;
            SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
            SetWindowPos(
                hwnd,
                HWND::default(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_SHOWWINDOW,
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
            SetParent(hwnd, HWND::default())
                .map_err(|e| format!("SetParent(desktop) failed: {e}"))?;
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