//! Brief §E: bottom-most click-through on the **main** window (no reparent).

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::wallpaper_subclass;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetParent, SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOWPOS,
        WM_WINDOWPOSCHANGING, GWL_EXSTYLE, HWND_BOTTOM, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_SHOWNA,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    const FALLBACK_SUBCLASS_ID: usize = 0x4c47_0001;
    static FALLBACK_ACTIVE: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn bottom_most_subclass(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _ref_data: usize,
    ) -> LRESULT {
        if msg == WM_WINDOWPOSCHANGING {
            let pos = &mut *(lparam.0 as *mut WINDOWPOS);
            if pos.hwndInsertAfter != HWND_BOTTOM {
                pos.hwndInsertAfter = HWND_BOTTOM;
            }
        }
        wallpaper_subclass::def_subclass_proc(hwnd, msg, wparam, lparam)
    }

    fn monitor_rect_for_window(hwnd: HWND) -> windows::Win32::Foundation::RECT {
        unsafe {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                info.rcMonitor
            } else {
                windows::Win32::Foundation::RECT {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1080,
                }
            }
        }
    }

    pub fn enter_main_fallback(main_hwnd: HWND, diag: &mut String) -> Result<(), String> {
        unsafe {
            let _ = SetParent(main_hwnd, None);
            let ex = GetWindowLongPtrW(main_hwnd, GWL_EXSTYLE) as u32;
            let ex = (ex & !0x8) | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0;
            let _ = SetWindowLongPtrW(main_hwnd, GWL_EXSTYLE, ex as isize);
            let monitor = monitor_rect_for_window(main_hwnd);
            let w = monitor.right - monitor.left;
            let h = monitor.bottom - monitor.top;
            let _ = SetWindowPos(
                main_hwnd,
                Some(HWND_BOTTOM),
                monitor.left,
                monitor.top,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            wallpaper_subclass::set_subclass(
                main_hwnd,
                Some(bottom_most_subclass),
                FALLBACK_SUBCLASS_ID,
            )?;
            FALLBACK_ACTIVE.store(true, Ordering::Relaxed);
            diag.push_str("fallback: main bottom-most + WM_WINDOWPOSCHANGING subclass\n");
            let _ = ShowWindow(main_hwnd, SW_SHOWNA);
            Ok(())
        }
    }

    pub fn leave_main_fallback(main_hwnd: HWND, diag: &mut String) {
        unsafe {
            if FALLBACK_ACTIVE.load(Ordering::Relaxed) {
                wallpaper_subclass::remove_subclass(
                    main_hwnd,
                    Some(bottom_most_subclass),
                    FALLBACK_SUBCLASS_ID,
                );
                FALLBACK_ACTIVE.store(false, Ordering::Relaxed);
                diag.push_str("fallback: removed main subclass\n");
            }
        }
    }

    pub fn is_main_fallback_active() -> bool {
        FALLBACK_ACTIVE.load(Ordering::Relaxed)
    }
}

#[cfg(windows)]
pub use imp::{enter_main_fallback, is_main_fallback_active, leave_main_fallback};

#[cfg(not(windows))]
pub fn enter_main_fallback(_: (), _: &mut String) -> Result<(), String> {
    Err("unsupported".into())
}

#[cfg(not(windows))]
pub fn leave_main_fallback(_: (), _: &mut String) {}

#[cfg(not(windows))]
pub fn is_main_fallback_active() -> bool {
    false
}