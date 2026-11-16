//! Main overlay window placement — never persist minimized or -32000 (off-screen) coords.

/// Rectangle in screen coordinates (platform-neutral helpers + tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectI32 {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

pub const OFF_SCREEN_THRESHOLD: i32 = -30_000;

pub fn placement_coords_sane(left: i32, top: i32) -> bool {
    left > OFF_SCREEN_THRESHOLD && top > OFF_SCREEN_THRESHOLD
}

/// Move `rect` into `work` when coords are off-screen or mostly outside the work area.
pub fn clamp_rect_to_work_area(rect: &mut RectI32, work: &RectI32) {
    let width = (rect.right - rect.left).max(320);
    let height = (rect.bottom - rect.top).max(240);
    let mut left = rect.left;
    let mut top = rect.top;
    if !placement_coords_sane(left, top) {
        left = work.left + 50;
        top = work.top + 50;
    }
    if left + 80 > work.right {
        left = work.right - width.min(520);
    }
    if top + 80 > work.bottom {
        top = work.bottom - height.min(440);
    }
    if left < work.left {
        left = work.left + 20;
    }
    if top < work.top {
        top = work.top + 20;
    }
    rect.left = left;
    rect.top = top;
    rect.right = left + width;
    rect.bottom = top + height;
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SetWindowPlacement, ShowWindow, WINDOWPLACEMENT, SW_RESTORE,
        SW_SHOW,
    };

    const WPF_RESTORETOMAXIMIZED: u32 = 0x2;
    pub(crate) use super::{clamp_rect_to_work_area, placement_coords_sane, RectI32, OFF_SCREEN_THRESHOLD};

    #[derive(Clone, Debug, Default)]
    pub struct SavedPlacement {
        pub placement: WINDOWPLACEMENT,
        pub valid: bool,
    }

    pub fn placement_sane(p: &WINDOWPLACEMENT) -> bool {
        let rc = p.rcNormalPosition;
        placement_coords_sane(rc.left, rc.top) && (p.showCmd as u32) != 2 // SW_SHOWMINIMIZED
    }

    pub fn save_main_placement(hwnd: HWND) -> SavedPlacement {
        unsafe {
            let mut placement = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            let ok = GetWindowPlacement(hwnd, &mut placement).is_ok();
            if !ok || !placement_sane(&placement) {
                return SavedPlacement::default();
            }
            SavedPlacement {
                placement,
                valid: true,
            }
        }
    }

    pub fn restore_main_placement(hwnd: HWND, saved: &SavedPlacement) -> Result<(), String> {
        if !saved.valid {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
            return Ok(());
        }
        let mut placement = saved.placement;
        placement.flags.0 &= !WPF_RESTORETOMAXIMIZED;
        placement.showCmd = 1; // SW_SHOWNORMAL
        if !placement_sane(&placement) {
            placement.rcNormalPosition.left = 100;
            placement.rcNormalPosition.top = 100;
        }
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            SetWindowPlacement(hwnd, &placement).map_err(|e| format!("SetWindowPlacement: {e}"))?;
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
        Ok(())
    }

    pub fn ensure_restored_visible(hwnd: HWND) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }
}

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
#[derive(Clone, Debug, Default)]
pub struct SavedPlacement;

#[cfg(not(windows))]
pub fn save_main_placement(_: ()) -> SavedPlacement {
    SavedPlacement
}

#[cfg(not(windows))]
pub fn restore_main_placement(_: (), _: &SavedPlacement) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_screen_threshold_constant() {
        assert!(OFF_SCREEN_THRESHOLD <= -30000);
    }

    #[test]
    fn placement_coords_sane_rejects_minimize_hide_coords() {
        assert!(!placement_coords_sane(-32_000, 100));
        assert!(!placement_coords_sane(100, -32_000));
        assert!(placement_coords_sane(100, 200));
    }

    #[test]
    fn clamp_rect_from_off_screen() {
        let work = RectI32 {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let mut rect = RectI32 {
            left: -32_000,
            top: -32_000,
            right: -31_480,
            bottom: -31_560,
        };
        clamp_rect_to_work_area(&mut rect, &work);
        assert!(placement_coords_sane(rect.left, rect.top));
        assert!(rect.left >= work.left);
        assert!(rect.top >= work.top);
    }

    #[test]
    fn clamp_rect_keeps_on_screen_window() {
        let work = RectI32 {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let mut rect = RectI32 {
            left: 400,
            top: 300,
            right: 920,
            bottom: 740,
        };
        clamp_rect_to_work_area(&mut rect, &work);
        assert_eq!(rect.left, 400);
        assert_eq!(rect.top, 300);
    }
}