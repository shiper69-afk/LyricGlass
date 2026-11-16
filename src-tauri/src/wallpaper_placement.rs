//! Main overlay window placement — never persist minimized or -32000 (off-screen) coords.

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, SetWindowPlacement, ShowWindow, WINDOWPLACEMENT, SW_RESTORE,
        SW_SHOW,
    };

    const WPF_RESTORETOMAXIMIZED: u32 = 0x2;
    pub(crate) const OFF_SCREEN_THRESHOLD: i32 = -30_000;

    #[derive(Clone, Debug, Default)]
    pub struct SavedPlacement {
        pub placement: WINDOWPLACEMENT,
        pub valid: bool,
    }

  pub fn placement_sane(p: &WINDOWPLACEMENT) -> bool {
        let rc = p.rcNormalPosition;
        rc.left > OFF_SCREEN_THRESHOLD
            && rc.top > OFF_SCREEN_THRESHOLD
            && (p.showCmd as u32) != 2 // SW_SHOWMINIMIZED
    }

    pub fn save_main_placement(hwnd: HWND) -> SavedPlacement {
        unsafe {
            let mut placement = WINDOWPLACEMENT {
                length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            let ok = GetWindowPlacement(hwnd, &mut placement).as_bool();
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
        placement.flags &= !WPF_RESTORETOMAXIMIZED;
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
    #[test]
    #[cfg(windows)]
    fn off_screen_threshold_constant() {
        assert!(super::imp::OFF_SCREEN_THRESHOLD < -30000);
    }
}