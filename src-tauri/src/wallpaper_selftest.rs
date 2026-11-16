//! `lyricglass.exe --wallpaper-selftest` — desktop attach probe without the full UI.
#![cfg(windows)]

use crate::wallpaper;
use crate::wallpaper_log;

pub fn run() -> i32 {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, ShowWindow, WNDCLASSW,
        CS_HREDRAW, CS_VREDRAW, SW_SHOW, WM_DESTROY,
    };

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_DESTROY {
            windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    let class_name: Vec<u16> = OsStr::new("LyricGlassWallpaperSelfTest")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let hwnd = unsafe {
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let title: Vec<u16> = OsStr::new("LyricGlass Wallpaper Selftest")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW,
            100,
            100,
            420,
            320,
            None,
            None,
            Some(hinst.into()),
            None,
        )
        .expect("CreateWindowExW");

        let _ = ShowWindow(hwnd, SW_SHOW);
        hwnd
    };

    let outcome = wallpaper::attach_wallpaper(hwnd, hwnd, "selftest");
    let code = match &outcome {
        Ok(()) => {
            let _ = wallpaper::detach_wallpaper(hwnd, hwnd, "selftest");
            0
        }
        Err(_) => 1,
    };

    unsafe {
        let _ = DestroyWindow(hwnd);
    }
    wallpaper_log::append_log_file(&format!(
        "selftest exit code={code} result={}",
        outcome.as_ref().map(|_| "ok").unwrap_or_else(|e| e.as_str())
    ));
    code
}