//! Click-through + z-order guard on the wallpaper webview HWND (brief §3).

#![cfg(windows)]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::wallpaper_subclass;
use parking_lot::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetClassNameW, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos,
    WINDOWPOS, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_WINDOWPOSCHANGING, GWL_EXSTYLE, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE, WS_EX_TRANSPARENT,
};

const SUBCLASS_ID: usize = 0x4c47_0003;
const HTTRANSPARENT: LRESULT = LRESULT(-1);
const MA_NOACTIVATE: isize = 3;

static INSTALLED: AtomicBool = AtomicBool::new(false);
static DEFVIEW_HWND: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct ZOrderCtx {
    progman: u64,
    defview: u64,
    workerw: u64,
    our_hwnd: u64,
}

static ZORDER_CTX: Mutex<Option<ZOrderCtx>> = Mutex::new(None);
static ZORDER_WATCH_STARTED: AtomicBool = AtomicBool::new(false);

pub fn set_defview_target(defview: HWND) {
    DEFVIEW_HWND.store(defview.0 as u64, Ordering::Relaxed);
}

pub fn install_wallpaper_input_guard(wallpaper_hwnd: HWND) {
    unsafe {
        apply_clickthrough_styles(wallpaper_hwnd);
        let _ = wallpaper_subclass::set_subclass(
            wallpaper_hwnd,
            Some(wallpaper_subclass_proc),
            SUBCLASS_ID,
        );
        apply_clickthrough_to_children(wallpaper_hwnd);
        INSTALLED.store(true, Ordering::Relaxed);
    }
}

pub fn remove_wallpaper_input_guard(wallpaper_hwnd: HWND) {
    unsafe {
        wallpaper_subclass::remove_subclass(
            wallpaper_hwnd,
            Some(wallpaper_subclass_proc),
            SUBCLASS_ID,
        );
        INSTALLED.store(false, Ordering::Relaxed);
    }
}

unsafe fn apply_clickthrough_styles(hwnd: HWND) {
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let ex = ex | WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0;
    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex as isize);
}

unsafe fn apply_clickthrough_to_children(root: HWND) {
    unsafe extern "system" fn enum_child(hwnd: HWND, _: LPARAM) -> windows::core::BOOL {
        apply_clickthrough_styles(hwnd);
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class);
        if len > 0 {
            let name = String::from_utf16_lossy(&class[..len as usize]);
            if name.contains("Chrome") || name.contains("WebView") || name == "Intermediate D3D Window"
            {
                apply_clickthrough_styles(hwnd);
            }
        }
        apply_clickthrough_to_children(hwnd);
        windows::core::BOOL::from(true)
    }
    let _ = EnumChildWindows(Some(root), Some(enum_child), LPARAM(0));
}

unsafe extern "system" fn wallpaper_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _ref_data: usize,
) -> LRESULT {
    match msg {
        WM_NCHITTEST => return HTTRANSPARENT,
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE),
        WM_WINDOWPOSCHANGING => {
            let defview = DEFVIEW_HWND.load(Ordering::Relaxed);
            if defview != 0 {
                let pos = &mut *(lparam.0 as *mut WINDOWPOS);
                pos.hwndInsertAfter = HWND(defview as _);
            }
        }
        _ => {}
    }
    wallpaper_subclass::def_subclass_proc(hwnd, msg, wparam, lparam)
}

pub fn start_zorder_watch(
    progman: HWND,
    defview: HWND,
    workerw: HWND,
    our_hwnd: HWND,
) {
    set_defview_target(defview);
    {
        let mut g = ZORDER_CTX.lock();
        *g = Some(ZOrderCtx {
            progman: progman.0 as u64,
            defview: defview.0 as u64,
            workerw: workerw.0 as u64,
            our_hwnd: our_hwnd.0 as u64,
        });
    }
    if ZORDER_WATCH_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let ctx = ZORDER_CTX.lock().clone();
            let Some(ctx) = ctx else {
                continue;
            };
            if !INSTALLED.load(Ordering::Relaxed) {
                continue;
            }
            let _ = check_zorder(&ctx);
        }
    });
}

pub fn stop_zorder_watch() {
    *ZORDER_CTX.lock() = None;
}

fn child_order_snapshot(progman: HWND) -> Vec<(u64, String)> {
    struct Ctx {
        out: Vec<(u64, String)>,
    }
    unsafe extern "system" fn enum_child(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        let ctx = &mut *(lparam.0 as *mut Ctx);
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class);
        let name = if len > 0 {
            String::from_utf16_lossy(&class[..len as usize])
        } else {
            "?".into()
        };
        ctx.out.push((hwnd.0 as u64, name));
        windows::core::BOOL::from(true)
    }
    let mut ctx = Ctx { out: Vec::new() };
    unsafe {
        let _ = EnumChildWindows(Some(progman), Some(enum_child), LPARAM(&mut ctx as *mut _ as _));
    }
    ctx.out
}

fn check_zorder(ctx: &ZOrderCtx) -> Result<(), String> {
    unsafe {
        let progman = HWND(ctx.progman as _);
        let defview = HWND(ctx.defview as _);
        let workerw = HWND(ctx.workerw as _);
        let our = HWND(ctx.our_hwnd as _);

        let before = child_order_snapshot(progman);
        let _ = SetWindowPos(our, Some(defview), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        if workerw.0 as u64 != 0 {
            use windows::Win32::UI::WindowsAndMessaging::HWND_BOTTOM;
            let _ = SetWindowPos(
                workerw,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        apply_clickthrough_to_children(our);

        let after = child_order_snapshot(progman);
        if before != after {
            let ts = time::OffsetDateTime::now_utc().unix_timestamp();
            let fmt = |v: &[(u64, String)]| {
                v.iter()
                    .enumerate()
                    .map(|(z, (h, c))| format!("z{z}:{c}=0x{h:x}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            crate::startup::step(&format!(
                "z-order change ts={ts} before=[{}] after=[{}]",
                fmt(&before),
                fmt(&after)
            ));
        }
    }
    Ok(())
}