//! Pure WorkerW / Progman desktop host selection (unit-tested without Win32).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopLayout {
    /// Classic: `SHELLDLL_DefView` under a top-level `WorkerW`; wallpaper host is the next top-level `WorkerW`.
    LegacyTopLevel,
    /// Win11 24H2+: `WorkerW` child of `Progman` (no top-level defview host).
    ProgmanChildWorkerW,
    /// Win11 24H2+ / 26H2: `SHELLDLL_DefView` and wallpaper `WorkerW` are siblings under `Progman`.
    ProgmanLayered,
}

#[derive(Debug, Clone)]
pub struct DesktopWindowInfo {
    pub hwnd: u64,
    pub class_name: String,
    pub parent_hwnd: u64,
    /// Direct child with class `SHELLDLL_DefView`.
    pub has_defview_child: bool,
    /// Sibling order index among parent's children (0 = topmost / first in `GW_CHILD` chain).
    pub z_order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperAttachPlan {
    pub layout: DesktopLayout,
    /// HWND passed to `SetParent` (our parent).
    pub parent_hwnd: u64,
    /// HWND passed as `hWndInsertAfter` in `SetWindowPos` (must not be bottom sentinel).
    pub z_insert_after_hwnd: u64,
    pub defview_hwnd: u64,
    pub wallpaper_workerw_hwnd: u64,
}

fn is_workerw(w: &DesktopWindowInfo) -> bool {
    w.class_name == "WorkerW"
}

fn is_defview(w: &DesktopWindowInfo) -> bool {
    w.class_name == "SHELLDLL_DefView"
}

fn is_progman(w: &DesktopWindowInfo) -> bool {
    w.class_name == "Progman"
}

/// Pick the HWND to `SetParent` into for wallpaper mode (top-level `EnumWindows` list only).
pub fn choose_wallpaper_parent(
    windows: &[DesktopWindowInfo],
    progman_hwnd: u64,
) -> Option<(u64, DesktopLayout)> {
    if let Some(hwnd) = choose_progman_child_workerw(windows, progman_hwnd) {
        return Some((hwnd, DesktopLayout::ProgmanChildWorkerW));
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
    let host_idx = top_level.iter().position(|w| w.has_defview_child)?;
    top_level.get(host_idx + 1).map(|w| w.hwnd)
}

/// Build attach plan from `Progman` direct children (`EnumChildWindows` order).
pub fn plan_progman_layered_attach(
    progman_children: &[DesktopWindowInfo],
    progman_hwnd: u64,
) -> Option<WallpaperAttachPlan> {
    let defview = progman_children.iter().find(|w| is_defview(w))?;
    let wallpaper_ww = progman_children
        .iter()
        .filter(|w| is_workerw(w) && !w.has_defview_child)
        .min_by_key(|w| w.z_order)
        .or_else(|| progman_children.iter().find(|w| is_workerw(w)))?;

    let wallpaper_workerw_hwnd = wallpaper_ww.map(|w| w.hwnd).unwrap_or(0);
    let z_insert_after = if wallpaper_workerw_hwnd != 0 {
        wallpaper_workerw_hwnd
    } else {
        // No WorkerW sibling: stack directly under DefView (insert after window below DefView).
        let defview_z = defview.z_order;
        progman_children
            .iter()
            .filter(|w| w.z_order > defview_z)
            .min_by_key(|w| w.z_order)
            .map(|w| w.hwnd)
            .unwrap_or(0)
    };

    Some(WallpaperAttachPlan {
        layout: DesktopLayout::ProgmanLayered,
        parent_hwnd: progman_hwnd,
        z_insert_after_hwnd: z_insert_after,
        defview_hwnd: defview.hwnd,
        wallpaper_workerw_hwnd,
    })
}

/// `siblings` ordered top → bottom (index 0 = topmost child).
pub fn verify_layered_z_order(
    siblings: &[(u64, usize)],
    our_hwnd: u64,
    defview_hwnd: u64,
    wallpaper_workerw_hwnd: u64,
) -> Result<(), &'static str> {
    let pos = |hwnd: u64| siblings.iter().find(|(h, _)| *h == hwnd).map(|(_, z)| *z);
    let our = pos(our_hwnd).ok_or("our window missing from progman children")?;
    let defview = pos(defview_hwnd).ok_or("defview missing from progman children")?;
    if wallpaper_workerw_hwnd != 0 {
        let ww = pos(wallpaper_workerw_hwnd).ok_or("wallpaper WorkerW missing from progman children")?;
        if !(defview < our && our < ww) {
            return Err("z-order: expected DefView < our < wallpaper WorkerW (top to bottom)");
        }
    } else if defview >= our {
        return Err("z-order: expected our window below DefView");
    }
    Ok(())
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
        assert_eq!(layout, DesktopLayout::ProgmanChildWorkerW);
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
        assert_eq!(layout, DesktopLayout::ProgmanChildWorkerW);
        assert_eq!(host, 0x210);
    }

    /// Ilan 26H2: DefView under Progman; top-level WorkerWs have no DefView — layered plan uses Progman + z-order.
    #[test]
    fn ilan_26340_progman_layered_plan() {
        let progman = 0x1029e;
        let children = vec![
            w(0xd001, "SHELLDLL_DefView", progman, false, 0),
            w(0xd002, "WorkerW", progman, false, 1),
        ];
        let plan = plan_progman_layered_attach(&children, progman).unwrap();
        assert_eq!(plan.layout, DesktopLayout::ProgmanLayered);
        assert_eq!(plan.parent_hwnd, progman);
        assert_eq!(plan.defview_hwnd, 0xd001);
        assert_eq!(plan.wallpaper_workerw_hwnd, 0xd002);
        assert_eq!(plan.z_insert_after_hwnd, 0xd002);

        let top_level = vec![
            w(0xcc0dee, "WorkerW", 0, false, 90),
            w(progman, "Progman", 0, true, 252),
        ];
        assert!(choose_wallpaper_parent(&top_level, progman).is_none());
    }

    #[test]
    fn layered_z_order_verify_accepts_middle_slot() {
        const LYRIC: u64 = 0x0e213a2;
        let siblings = vec![(0xd001, 0), (LYRIC, 1), (0xd002, 2)];
        verify_layered_z_order(&siblings, LYRIC, 0xd001, 0xd002).unwrap();
    }

    #[test]
    fn layered_z_order_rejects_behind_wallpaper() {
        const LYRIC: u64 = 0x0e213a2;
        let siblings = vec![(0xd001, 0), (0xd002, 1), (LYRIC, 2)];
        assert!(verify_layered_z_order(&siblings, LYRIC, 0xd001, 0xd002).is_err());
    }
}