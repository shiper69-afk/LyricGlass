//! comctl32 window subclasses (brief §E WM_WINDOWPOSCHANGING).

#![cfg(windows)]

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

pub type SubclassProc =
    Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>;

#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(
        hwnd: HWND,
        pfn_subclass: SubclassProc,
        u_id_subclass: usize,
        dw_ref_data: usize,
    ) -> BOOL;
    fn RemoveWindowSubclass(
        hwnd: HWND,
        pfn_subclass: SubclassProc,
        u_id_subclass: usize,
    ) -> BOOL;
    fn DefSubclassProc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

pub unsafe fn set_subclass(
    hwnd: HWND,
    proc: SubclassProc,
    id: usize,
) -> Result<(), String> {
    if SetWindowSubclass(hwnd, proc, id, 0).as_bool() {
        Ok(())
    } else {
        Err("SetWindowSubclass failed".into())
    }
}

pub unsafe fn remove_subclass(hwnd: HWND, proc: SubclassProc, id: usize) {
    let _ = RemoveWindowSubclass(hwnd, proc, id);
}

pub unsafe fn def_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefSubclassProc(hwnd, msg, wparam, lparam)
}