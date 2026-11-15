//! Pure WorkerW / Progman desktop host selection (unit-tested without Win32).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopLayout {
    /// Classic: `SHELLDLL_DefView` under a top-level `WorkerW`; wallpaper host is the next top-level `WorkerW`.
    LegacyTopLevel,
    /// Win11 24H2+: `WorkerW` children of `Progman`; host is a `WorkerW` child without `DefView`.
    ProgmanChild,
}

#[derive(Debug, Clone)]
pub struct DesktopWindowInfo {
    pub hwnd: u64,
    pub class_name: String,
    pub parent_hwnd: u64,
    /// Direct child with class `SHELLDLL_DefView`.
    pub has_defview_child: bool,
    /// `EnumWindows` visit index (lower = higher in z-order for top-level).
    pub z_order: usize,
}

fn is_workerw(w: &DesktopWindowInfo) -> bool {
    w.class_name == "WorkerW"
}

fn is_progman(w: &DesktopWindowInfo) -> bool {
    w.class_name == "Progman"
}

/// Pick the HWND to `SetParent` into for wallpaper mode.
pub fn choose_wallpaper_parent(
    windows: &[DesktopWindowInfo],
    progman_hwnd: u64,
) -> Option<(u64, DesktopLayout)> {
    if let Some(hwnd) = choose_progman_child_workerw(windows, progman_hwnd) {
        return Some((hwnd, DesktopLayout::ProgmanChild));
    }
    if let Some(hwnd) = choose_legacy_workerw_after_defview(windows) {
        return Some((hwnd, DesktopLayout::LegacyTopLevel));
    }
    None
}

fn choose_progman_child_workerw(
    windows: &[DesktopWindowInfo],
    progman_hwnd: u64,
) -> Option<u64> {
    windows
        .iter()
        .filter(|w| is_workerw(w) && w.parent_hwnd == progman_hwnd && !w.has_defview_child)
        .map(|w| w.hwnd)
        .next()
}

/// Ordered top-level `WorkerW` list: host is the entry immediately after the one hosting `DefView`.
fn choose_legacy_workerw_after_defview(windows: &[DesktopWindowInfo]) -> Option<u64> {
    let mut top_level: Vec<&DesktopWindowInfo> = windows
        .iter()
        .filter(|w| is_workerw(w) && w.parent_hwnd == 0)
        .collect();
    top_level.sort_by_key(|w| w.z_order);
    let host_idx = top_level
        .iter()
        .position(|w| w.has_defview_child)?;
    top_level.get(host_idx + 1).map(|w| w.hwnd)
}

pub fn detect_layout(windows: &[DesktopWindowInfo], progman_hwnd: u64) -> Option<DesktopLayout> {
    choose_wallpaper_parent(windows, progman_hwnd).map(|(_, l)| l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(
        hwnd: u64,
        class: &str,
        parent: u64,
        defview: bool,
        z_order: usize,
    ) -> DesktopWindowInfo {
        DesktopWindowInfo {
            hwnd,
            class_name: class.into(),
            parent_hwnd: parent,
            has_defview_child: defview,
            z_order,
        }
    }

    #[test]
    fn legacy_picks_workerw_after_defview_host() {
        let progman = 0x100;
        let list = vec![
            w(progman, "Progman", 0, false, 0),
            w(0x200, "WorkerW", 0, true, 1),
            w(0x300, "WorkerW", 0, false, 2),
            w(0x400, "WorkerW", 0, false, 3),
        ];
        let (host, layout) = choose_wallpaper_parent(&list, progman).unwrap();
        assert_eq!(layout, DesktopLayout::LegacyTopLevel);
        assert_eq!(host, 0x300);
    }

    #[test]
    fn progman_child_prefers_workerw_without_defview() {
        let progman = 0x100;
        let list = vec![
            w(progman, "Progman", 0, false, 0),
            w(0x210, "WorkerW", progman, true, 1),
            w(0x220, "WorkerW", progman, false, 2),
        ];
        let (host, layout) = choose_wallpaper_parent(&list, progman).unwrap();
        assert_eq!(layout, DesktopLayout::ProgmanChild);
        assert_eq!(host, 0x220);
    }

    #[test]
    fn progman_child_wins_when_both_patterns_present() {
        let progman = 0x100;
        let list = vec![
            w(progman, "Progman", 0, false, 0),
            w(0x210, "WorkerW", progman, false, 1),
            w(0x300, "WorkerW", 0, true, 2),
            w(0x310, "WorkerW", 0, false, 3),
        ];
        let (host, layout) = choose_wallpaper_parent(&list, progman).unwrap();
        assert_eq!(layout, DesktopLayout::ProgmanChild);
        assert_eq!(host, 0x210);
    }
}