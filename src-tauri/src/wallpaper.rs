use crate::startup;
use crate::wallpaper_log;
use crate::wallpaper_select::{
    choose_wallpaper_parent, plan_progman_layered_attach, verify_progman_layered_state,
    DesktopLayout, DesktopWindowInfo,
    ProgmanAttachPath, ProgmanAttachVerifyInput, WallpaperAttachPlan,
};
use parking_lot::Mutex;
use std::sync::LazyLock;

pub fn last_attach_error() -> Option<String> {
    wallpaper_log::last_attempt().and_then(|a| a.error)
}

pub fn last_attach_diagnostics() -> Option<String> {
    wallpaper_log::last_attempt().map(|a| a.trace)
}

fn finish_attempt(
    trigger: &str,
    outcome: &str,
    attached_after: bool,
    error: Option<String>,
    trace: String,
) {
    startup::step(&format!("wallpaper: {outcome} trigger={trigger} attached={attached_after}"));
    wallpaper_log::record_attempt(trigger, outcome, attached_after, error.clone(), trace.clone());
    if error.is_some() {
        startup::log_diagnostic(&format!("WALLPAPER ATTACH\n{trace}"));
    }
}

#[derive(Debug, Default, Clone)]
pub struct OverlayGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Clone)]
struct WallpaperRestore {
    attached: bool,
    overlay: OverlayGeometry,
    #[cfg(windows)]
    previous_style: isize,
    #[cfg(windows)]
    previous_ex_style: isize,
    #[cfg(windows)]
    host_hwnd: u64,
    #[cfg(windows)]
    attach_path: Option<ProgmanAttachPath>,
}

static WALLPAPER_STATE: LazyLock<Mutex<WallpaperRestore>> =
    LazyLock::new(|| Mutex::new(WallpaperRestore::default()));

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::{w, BOOL};
    use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, RECT, SetLastError, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, RedrawWindow, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
        RDW_ALLCHILDREN, RDW_INVALIDATE, RDW_UPDATENOW,
    };
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, EnumChildWindows, EnumWindows,
        FindWindowExW, FindWindowW, GetClassNameW, GetParent, GetWindow, GetWindowLongPtrW,
        GetWindowRect, IsWindow, IsWindowVisible, RegisterClassExW, SendMessageTimeoutW,
        SendMessageW, SetLayeredWindowAttributes, SetParent, SetWindowLongPtrW, SetWindowPos,
        ShowWindow, WNDCLASSEXW, GWL_EXSTYLE, GWL_STYLE, GW_CHILD, GW_HWNDNEXT, HWND_NOTOPMOST,
        SHOW_WINDOW_CMD,
        LWA_ALPHA, SMTO_ABORTIFHUNG, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOREDRAW,
        SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_SHOW, WM_SETREDRAW,
    };

    const PROGMAN_DESKTOP_MSG: u32 = 0x052C;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_POPUP: u32 = 0x8000_0000;
    const WS_CAPTION: u32 = 0x00C0_0000;
    const WS_THICKFRAME: u32 = 0x0004_0000;
    const WS_MINIMIZEBOX: u32 = 0x0002_0000;
    const WS_MAXIMIZEBOX: u32 = 0x0001_0000;
    const WS_SYSMENU: u32 = 0x0008_0000;
    const WS_BORDER: u32 = 0x0080_0000;
    const WS_DLGFRAME: u32 = 0x0040_0000;
    const WS_EX_TOPMOST: u32 = 0x0000_0008;
    const WS_EX_LAYERED: u32 = 0x0008_0000;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
    const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;
    const WS_EX_NOREDIRECTIONBITMAP: u32 = 0x0020_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const SW_SHOWNA: i32 = 8;
    const ERROR_CLASS_ALREADY_EXISTS: u32 = 1410;

    fn hwnd_to_u64(hwnd: HWND) -> u64 {
        hwnd.0 as usize as u64
    }

    fn hwnd_valid(hwnd: HWND) -> bool {
        !hwnd.0.is_null() && unsafe { IsWindow(Some(hwnd)).as_bool() }
    }

    fn progman() -> Option<HWND> {
        let hwnd = unsafe { FindWindowW(w!("Progman"), None).ok()? };
        hwnd_valid(hwnd).then_some(hwnd)
    }

    fn last_error_code() -> u32 {
        unsafe { GetLastError().0 }
    }

    fn clear_last_error() {
        unsafe {
            let _ = SetLastError(windows::Win32::Foundation::WIN32_ERROR(0));
        }
    }

    fn set_window_long_ptr_logged(
        hwnd: HWND,
        index: windows::Win32::UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX,
        value: isize,
        label: &str,
        diag: &mut String,
    ) {
        unsafe {
            clear_last_error();
            let prev = SetWindowLongPtrW(hwnd, index, value);
            let err = last_error_code();
            let now = GetWindowLongPtrW(hwnd, index);
            diag.push_str(&format!(
                "{label}: SetWindowLongPtr({index:?}) value=0x{value:x} prev=0x{prev:x} now=0x{now:x} lastError={err}\n"
            ));
        }
    }

    fn log_style_bits(kind: &str, value: u32, diag: &mut String) {
        let visible = value & WS_VISIBLE != 0;
        let child = value & WS_CHILD != 0;
        diag.push_str(&format!(
            "{kind} bits: 0x{value:x} CHILD={child} VISIBLE={visible}\n"
        ));
    }

    fn log_exstyle_bits(kind: &str, value: u32, diag: &mut String) {
        let topmost = value & WS_EX_TOPMOST != 0;
        let layered = value & WS_EX_LAYERED != 0;
        let noredir = value & WS_EX_NOREDIRECTIONBITMAP != 0;
        let noactivate = value & WS_EX_NOACTIVATE != 0;
        let toolwin = value & WS_EX_TOOLWINDOW != 0;
        diag.push_str(&format!(
            "{kind} bits: 0x{value:x} TOPMOST={topmost} LAYERED={layered} NOREDIR={noredir} NOACTIVATE={noactivate} TOOLWINDOW={toolwin}\n"
        ));
    }

    fn force_clear_topmost(hwnd: HWND, diag: &mut String) {
        unsafe {
            clear_last_error();
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_NOTOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            diag.push_str(&format!(
                "SetWindowPos(HWND_NOTOPMOST) lastError={}\n",
                last_error_code()
            ));
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            if ex & WS_EX_TOPMOST != 0 {
                let cleared = ex & !WS_EX_TOPMOST;
                set_window_long_ptr_logged(
                    hwnd,
                    GWL_EXSTYLE,
                    cleared as isize,
                    "exstyle clear TOPMOST",
                    diag,
                );
            }
        }
    }

    fn set_redraw(hwnd: HWND, on: bool, diag: &mut String) {
        unsafe {
            let wp = WPARAM(if on { 1 } else { 0 } as usize);
            let _ = SendMessageW(hwnd, WM_SETREDRAW, Some(wp), Some(LPARAM(0)));
            diag.push_str(&format!("WM_SETREDRAW={on} hwnd=0x{:x}\n", hwnd_to_u64(hwnd)));
        }
    }

    const HOST_CLASS: &str = "LyricGlassWallpaperHost";
    const FAKE_PROGMAN_CLASS: &str = "LgFakeProgman";
    const FAKE_DEFVIEW_CLASS: &str = "LgFakeShellDefView";
    const FAKE_WORKERW_CLASS: &str = "LgFakeWorkerW";
    const FAKE_CONTENT_CLASS: &str = "LgFakeContent";

    #[repr(C)]
    struct RtlOsVersionInfo {
        dw_os_version_info_size: u32,
        dw_major_version: u32,
        dw_minor_version: u32,
        dw_build_number: u32,
        dw_platform_id: u32,
        sz_csd_version: [u16; 128],
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn RtlGetVersion(info: *mut RtlOsVersionInfo) -> i32;
    }

    fn windows_build_label() -> String {
        unsafe {
            let mut v = RtlOsVersionInfo {
                dw_os_version_info_size: std::mem::size_of::<RtlOsVersionInfo>() as u32,
                dw_major_version: 0,
                dw_minor_version: 0,
                dw_build_number: 0,
                dw_platform_id: 0,
                sz_csd_version: [0; 128],
            };
            let status = RtlGetVersion(&mut v);
            format!(
                "RtlGetVersion status={status} {}.{}.{} build {}",
                v.dw_major_version,
                v.dw_minor_version,
                v.dw_build_number,
                v.dw_build_number
            )
        }
    }

    fn spawn_desktop_workerw(diag: &mut String) -> Result<(), String> {
        let progman = progman().ok_or_else(|| "Progman not found".to_string())?;
        diag.push_str(&format!("progman_hwnd=0x{:x}\n", hwnd_to_u64(progman)));

        let variants: [(isize, isize); 4] = [(0xD, 0), (0xD, 1), (0, 0), (1, 0)];
        for (attempt, (wp, lp)) in variants.iter().enumerate() {
            let mut result = 0usize;
            let r = unsafe {
                SendMessageTimeoutW(
                    progman,
                    PROGMAN_DESKTOP_MSG,
                    WPARAM(*wp as usize),
                    LPARAM(*lp),
                    SMTO_ABORTIFHUNG,
                    1500,
                    Some(&mut result),
                )
            };
            diag.push_str(&format!(
                "spawn attempt {attempt}: msg=0x{PROGMAN_DESKTOP_MSG:x} wParam=0x{wp:x} lParam=0x{lp:x} ok={} result=0x{result:x} lastError={}\n",
                r.0 != 0,
                last_error_code()
            ));
        }
        Ok(())
    }

    struct EnumCollect {
        z: usize,
        windows: Vec<DesktopWindowInfo>,
    }

    unsafe extern "system" fn enum_collect_top(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !hwnd_valid(hwnd) {
            return BOOL::from(true);
        }
        let ctx = &mut *(lparam.0 as *mut EnumCollect);
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class);
        let class_name = if len > 0 {
            String::from_utf16_lossy(&class[..len as usize])
        } else {
            String::new()
        };
        let parent = GetParent(hwnd).unwrap_or(HWND::default());
        let parent_hwnd = if parent.0.is_null() {
            0
        } else {
            hwnd_to_u64(parent)
        };
        let defview =
            FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None).ok();
        let has_defview_child = defview.map(hwnd_valid).unwrap_or(false);
        ctx.windows.push(DesktopWindowInfo {
            hwnd: hwnd_to_u64(hwnd),
            class_name,
            parent_hwnd,
            has_defview_child,
            z_order: ctx.z,
        });
        ctx.z += 1;
        BOOL::from(true)
    }

    fn collect_desktop_windows() -> Vec<DesktopWindowInfo> {
        let mut ctx = EnumCollect {
            z: 0,
            windows: Vec::new(),
        };
        unsafe {
            let _ = EnumWindows(Some(enum_collect_top), LPARAM(&mut ctx as *mut _ as isize));
        }
        ctx.windows
    }

    fn wait_for_workerw(diag: &mut String) -> Result<(HWND, DesktopLayout), String> {
        let progman = progman().ok_or_else(|| "Progman missing after spawn".to_string())?;
        for retry in 0..12 {
            let list = collect_desktop_windows();
            if retry == 0 || retry == 11 {
                for w in &list {
                    if w.class_name == "WorkerW" || w.class_name == "Progman" {
                        diag.push_str(&format!(
                            "  hwnd=0x{:x} class={} parent=0x{:x} defview={} z={}\n",
                            w.hwnd, w.class_name, w.parent_hwnd, w.has_defview_child, w.z_order
                        ));
                    }
                }
            }
            if let Some((host, layout)) =
                choose_wallpaper_parent(&list, hwnd_to_u64(progman))
            {
                let hwnd = HWND(host as *mut core::ffi::c_void);
                if hwnd_valid(hwnd) {
                    diag.push_str(&format!(
                        "workerw retry {retry}: host=0x{host:x} layout={layout:?}\n"
                    ));
                    return Ok((hwnd, layout));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
        Err("WorkerW host not found after retries".into())
    }

    fn parent_chain(hwnd: HWND) -> String {
        let mut parts = Vec::new();
        let mut cur = hwnd;
        for _ in 0..8 {
            if !hwnd_valid(cur) {
                break;
            }
            let mut class = [0u16; 64];
            let len = unsafe { GetClassNameW(cur, &mut class) };
            let name = if len > 0 {
                String::from_utf16_lossy(&class[..len as usize])
            } else {
                "?".into()
            };
            parts.push(format!("0x{:x}({name})", hwnd_to_u64(cur)));
            let parent = unsafe { GetParent(cur) }.unwrap_or(HWND::default());
            if parent.0.is_null() || parent == cur {
                break;
            }
            cur = parent;
        }
        parts.join(" -> ")
    }

    fn log_shell_defview(diag: &mut String, progman: Option<HWND>) {
        let mut found = false;
        if let Some(progman) = progman {
            let dv = unsafe {
                FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok()
            };
            if let Some(hwnd) = dv.filter(|h| hwnd_valid(*h)) {
                found = true;
                diag.push_str(&format!(
                    "SHELLDLL_DefView (progman child) hwnd=0x{:x} parent=0x{:x} chain={}\n",
                    hwnd_to_u64(hwnd),
                    hwnd_to_u64(progman),
                    parent_chain(hwnd)
                ));
            }
        }
        let dv_top = unsafe { FindWindowExW(None, None, w!("SHELLDLL_DefView"), None).ok() };
        if let Some(hwnd) = dv_top.filter(|h| hwnd_valid(*h)) {
            let parent = unsafe { GetParent(hwnd) }.unwrap_or(HWND::default());
            diag.push_str(&format!(
                "SHELLDLL_DefView (desktop) hwnd=0x{:x} parent=0x{:x} chain={}\n",
                hwnd_to_u64(hwnd),
                hwnd_to_u64(parent),
                parent_chain(hwnd)
            ));
            found = true;
        }
        if !found {
            diag.push_str("SHELLDLL_DefView: not found\n");
        }
    }

    struct ChildCollect {
        parent_hwnd: u64,
        z: usize,
        windows: Vec<DesktopWindowInfo>,
    }

    unsafe extern "system" fn enum_child_collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !hwnd_valid(hwnd) {
            return BOOL::from(true);
        }
        let ctx = &mut *(lparam.0 as *mut ChildCollect);
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class);
        let class_name = if len > 0 {
            String::from_utf16_lossy(&class[..len as usize])
        } else {
            String::new()
        };
        let defview = FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None).ok();
        let has_defview_child = defview.map(hwnd_valid).unwrap_or(false);
        ctx.windows.push(DesktopWindowInfo {
            hwnd: hwnd_to_u64(hwnd),
            class_name,
            parent_hwnd: ctx.parent_hwnd,
            has_defview_child,
            z_order: ctx.z,
        });
        ctx.z += 1;
        BOOL::from(true)
    }

    fn progman_child_sibling_order(progman: HWND) -> Vec<(u64, String, usize)> {
        let mut out = Vec::new();
        unsafe {
            let mut child = GetWindow(progman, GW_CHILD).unwrap_or(HWND::default());
            let mut z = 0usize;
            while hwnd_valid(child) {
                let mut class = [0u16; 64];
                let len = GetClassNameW(child, &mut class);
                let class_name = if len > 0 {
                    String::from_utf16_lossy(&class[..len as usize])
                } else {
                    String::new()
                };
                out.push((hwnd_to_u64(child), class_name, z));
                z += 1;
                child = GetWindow(child, GW_HWNDNEXT).unwrap_or(HWND::default());
            }
        }
        out
    }

    fn enumerate_progman_children(progman: HWND, diag: &mut String) -> Vec<DesktopWindowInfo> {
        let mut ctx = ChildCollect {
            parent_hwnd: hwnd_to_u64(progman),
            z: 0,
            windows: Vec::new(),
        };
        unsafe {
            let _ = EnumChildWindows(Some(progman), Some(enum_child_collect), LPARAM(
                &mut ctx as *mut _ as isize,
            ));
        }
        let sibling_order = progman_child_sibling_order(progman);
        for (hwnd, class, z) in &sibling_order {
            diag.push_str(&format!(
                "  progman child hwnd=0x{hwnd:x} class={class} sibling_z={z}\n"
            ));
        }
        for w in &mut ctx.windows {
            if let Some((_, _, z)) = sibling_order.iter().find(|(h, _, _)| *h == w.hwnd) {
                w.z_order = *z;
            }
        }
        ctx.windows.sort_by_key(|w| w.z_order);
        ctx.windows
    }

    fn apply_wallpaper_styles(
        hwnd: HWND,
        snapshot: &WallpaperRestore,
        diag: &mut String,
        preserve_noredirection: bool,
    ) -> Result<(), String> {
        unsafe {
            let style_before = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let ex_before = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            diag.push_str(&format!(
                "style_before=0x{style_before:x} ex_before=0x{ex_before:x}\n"
            ));

            let style = style_before as u32;
            let strip = WS_POPUP
                | WS_CAPTION
                | WS_THICKFRAME
                | WS_MINIMIZEBOX
                | WS_MAXIMIZEBOX
                | WS_SYSMENU
                | WS_BORDER
                | WS_DLGFRAME;
            let style = (style & !strip) | WS_CHILD;
            log_style_bits("style requested", style, diag);
            set_window_long_ptr_logged(hwnd, GWL_STYLE, style as isize, "style", diag);

            let mut ex = ex_before as u32;
            ex &= !WS_EX_TOPMOST;
            ex &= !WS_EX_APPWINDOW;
            ex &= !WS_EX_LAYERED;
            ex |= WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
            if preserve_noredirection {
                ex |= WS_EX_NOREDIRECTIONBITMAP;
            }
            log_exstyle_bits("exstyle requested", ex, diag);
            set_window_long_ptr_logged(hwnd, GWL_EXSTYLE, ex as isize, "exstyle", diag);
            force_clear_topmost(hwnd, diag);

            let style_after = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let ex_after = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            diag.push_str(&format!(
                "style_after=0x{style_after:x} ex_after=0x{ex_after:x}\n"
            ));
            log_style_bits("style actual", style_after as u32, diag);
            log_exstyle_bits("exstyle actual", ex_after, diag);
            if ex_after != ex {
                diag.push_str(&format!(
                    "exstyle mismatch: requested=0x{ex:x} actual=0x{ex_after:x} delta=0x{:x}\n",
                    ex ^ ex_after
                ));
            }
            let _ = snapshot;
        }
        Ok(())
    }

    type HostWndProc = Option<
        unsafe extern "system" fn(
            HWND,
            u32,
            WPARAM,
            LPARAM,
        ) -> windows::Win32::Foundation::LRESULT,
    >;

    unsafe fn register_wnd_class(name: &str, proc: HostWndProc) -> Result<(), String> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        let class_name: Vec<u16> = OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: proc,
            hInstance: hinst.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        clear_last_error();
        let atom = RegisterClassExW(&wc);
        let err = last_error_code();
        if atom == 0 && err != ERROR_CLASS_ALREADY_EXISTS {
            return Err(format!("RegisterClassExW({name}): lastError={err}"));
        }
        Ok(())
    }

    fn register_host_class(diag: &mut String) -> Result<(), String> {
        unsafe {
            register_wnd_class(HOST_CLASS, Some(host_wnd_proc))?;
        }
        diag.push_str("host class registered (RegisterClassExW)\n");
        Ok(())
    }

    unsafe extern "system" fn host_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> windows::Win32::Foundation::LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn create_layered_host(
        progman: HWND,
        plan: &WallpaperAttachPlan,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        defer_show: bool,
        diag: &mut String,
    ) -> Result<HWND, String> {
        register_host_class(diag)?;
        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            use windows::core::PCWSTR;

            let class_name: Vec<u16> = OsStr::new(HOST_CLASS)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let hinst = GetModuleHandleW(None).unwrap_or_default();

            clear_last_error();
            let host = match CreateWindowExW(
                windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE(
                    WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                ),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(class_name.as_ptr()),
                windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(WS_CHILD),
                x,
                y,
                w,
                h,
                Some(progman),
                None,
                Some(hinst.into()),
                None,
            ) {
                Ok(h) if hwnd_valid(h) => h,
                Ok(h) => {
                    return Err(format!(
                        "CreateWindowExW(host): NULL hwnd=0x{:x} lastError={}",
                        hwnd_to_u64(h),
                        last_error_code()
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "CreateWindowExW(host): {e} lastError={}",
                        last_error_code()
                    ));
                }
            };

            diag.push_str(&format!(
                "path B: host hwnd=0x{:x} parent=0x{:x}\n",
                hwnd_to_u64(host),
                hwnd_to_u64(progman)
            ));

            let layered_r = SetLayeredWindowAttributes(
                host,
                windows::Win32::Foundation::COLORREF(0),
                255,
                LWA_ALPHA,
            );
            diag.push_str(&format!(
                "SetLayeredWindowAttributes(host) ret={layered_r:?} lastError={}\n",
                last_error_code()
            ));
            layered_r.map_err(|e| format!("SetLayeredWindowAttributes(host): {e}"))?;

            let insert_after = HWND(plan.z_insert_after_hwnd as *mut core::ffi::c_void);
            let flags = SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOREDRAW;
            let flags = if defer_show {
                flags
            } else {
                flags | SWP_SHOWWINDOW
            };
            SetWindowPos(host, Some(insert_after), x, y, w, h, flags)
                .map_err(|e| format!("SetWindowPos(host): {e}"))?;
            Ok(host)
        }
    }

    fn monitor_rect_for_window(hwnd: HWND) -> (RECT, u32) {
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let dpi = if dpi == 0 { 96 } else { dpi };
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let rect = if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            info.rcMonitor
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        };
        (rect, dpi)
    }

    fn child_screen_rect(parent: HWND, hwnd: HWND) -> (i32, i32, i32, i32) {
        let (monitor, dpi) = monitor_rect_for_window(hwnd);
        let width = monitor.right - monitor.left;
        let height = monitor.bottom - monitor.top;
        let parent_rect = unsafe {
            let mut r = RECT::default();
            let _ = GetWindowRect(parent, &mut r);
            r
        };
        // Position relative to parent client (WorkerW origin is screen coords).
        let x = monitor.left - parent_rect.left;
        let y = monitor.top - parent_rect.top;
        let _ = dpi;
        (x, y, width, height)
    }

    fn rollback_attach(hwnd: HWND, restore: &WallpaperRestore, diag: &mut String) {
        unsafe {
            set_redraw(hwnd, false, diag);
            if restore.host_hwnd != 0 {
                let _ = SetParent(hwnd, None);
                let host = HWND(restore.host_hwnd as *mut core::ffi::c_void);
                let _ = DestroyWindow(host);
                diag.push_str(&format!("destroyed host 0x{:x}\n", restore.host_hwnd));
            } else {
                let _ = SetParent(hwnd, None);
            }
            set_window_long_ptr_logged(
                hwnd,
                GWL_STYLE,
                restore.previous_style,
                "rollback style",
                diag,
            );
            set_window_long_ptr_logged(
                hwnd,
                GWL_EXSTYLE,
                restore.previous_ex_style,
                "rollback exstyle",
                diag,
            );
            let geo = &restore.overlay;
            let _ = SetWindowPos(
                hwnd,
                None,
                geo.x,
                geo.y,
                geo.width as i32,
                geo.height as i32,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            set_redraw(hwnd, true, diag);
        }
    }

    fn finalize_progman_show(
        hwnd: HWND,
        defview: HWND,
        host: Option<HWND>,
        diag: &mut String,
    ) -> Result<(), String> {
        unsafe {
            force_clear_topmost(hwnd, diag);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 | WS_VISIBLE;
            set_window_long_ptr_logged(hwnd, GWL_STYLE, style as isize, "style show", diag);
            let _ = ShowWindow(hwnd, SHOW_WINDOW_CMD(SW_SHOWNA));
            diag.push_str("ShowWindow(SW_SHOWNA) content\n");

            let _ = SetWindowPos(
                hwnd,
                Some(defview),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            diag.push_str(&format!(
                "SetWindowPos z-order insert_below DefView=0x{:x} lastError={}\n",
                hwnd_to_u64(defview),
                last_error_code()
            ));

            if let Some(host) = host {
                force_clear_topmost(host, diag);
                let hs = GetWindowLongPtrW(host, GWL_STYLE) as u32 | WS_VISIBLE;
                set_window_long_ptr_logged(host, GWL_STYLE, hs as isize, "host style show", diag);
                let _ = ShowWindow(host, SHOW_WINDOW_CMD(SW_SHOWNA));
                let _ = SetWindowPos(
                    host,
                    Some(defview),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                diag.push_str(&format!(
                    "host show+z-order host=0x{:x} lastError={}\n",
                    hwnd_to_u64(host),
                    last_error_code()
                ));
            }

            set_redraw(hwnd, true, diag);
            let _ = RedrawWindow(
                Some(hwnd),
                None,
                None,
                RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
            );
            diag.push_str("RedrawWindow content\n");
        }
        Ok(())
    }

    fn reparent_fullscreen(
        hwnd: HWND,
        parent: HWND,
        host_label: &str,
        layout: Option<DesktopLayout>,
        snapshot: &WallpaperRestore,
        diag: &mut String,
        z_insert_after: Option<HWND>,
        defer_show: bool,
        fill_parent: bool,
    ) -> Result<(), String> {
        unsafe {
            let sp = SetParent(hwnd, Some(parent));
            diag.push_str(&format!(
                "SetParent({host_label}) ret={sp:?} lastError={}\n",
                last_error_code()
            ));
            sp.map_err(|e| format!("SetParent({host_label}): {e}"))?;

            apply_wallpaper_styles(hwnd, snapshot, diag, true)?;

            let full = child_screen_rect(parent, hwnd);
            let (x, y, w, h) = if fill_parent {
                (0, 0, full.2, full.3)
            } else {
                full
            };
            let (_, dpi) = monitor_rect_for_window(hwnd);
            let z_label = z_insert_after
                .map(|z| format!("0x{:x}", hwnd_to_u64(z)))
                .unwrap_or_else(|| "unchanged".into());
            diag.push_str(&format!(
                "host={host_label} layout={layout:?} parent_hwnd=0x{:x} dpi={dpi}\nsetpos x={x} y={y} w={w} h={h} insert_after={z_label} defer_show={defer_show}\n",
                hwnd_to_u64(parent)
            ));

            let mut flags = SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOREDRAW;
            if !defer_show {
                flags |= SWP_SHOWWINDOW;
            }
            SetWindowPos(hwnd, z_insert_after, x, y, w, h, flags)
                .map_err(|e| format!("SetWindowPos({host_label}): {e} (lastError={})", last_error_code()))?;

            let got_parent = GetParent(hwnd).unwrap_or(HWND::default());
            diag.push_str(&format!(
                "verify GetParent=0x{:x} expected=0x{:x}\n",
                hwnd_to_u64(got_parent),
                hwnd_to_u64(parent)
            ));
            if got_parent != parent {
                return Err(format!(
                    "parent verify failed: got 0x{:x} want 0x{:x}",
                    hwnd_to_u64(got_parent),
                    hwnd_to_u64(parent)
                ));
            }
            diag.push_str(&format!("our_parent_chain: {}\n", parent_chain(hwnd)));
        }
        Ok(())
    }

    fn collect_verify_input(
        hwnd: HWND,
        plan: &WallpaperAttachPlan,
        path: ProgmanAttachPath,
        host_hwnd: u64,
        diag: &mut String,
    ) -> ProgmanAttachVerifyInput {
        unsafe {
            let progman = HWND(plan.parent_hwnd as *mut core::ffi::c_void);
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            let rect_w = rect.right - rect.left;
            let rect_h = rect.bottom - rect.top;
            let our_parent = GetParent(hwnd).unwrap_or(HWND::default());
            let (z_order_hwnd, our_parent_expected) = match path {
                ProgmanAttachPath::A => (hwnd_to_u64(hwnd), plan.parent_hwnd),
                ProgmanAttachPath::B => (host_hwnd, host_hwnd),
            };
            let z_order_parent_hwnd = GetParent(HWND(z_order_hwnd as *mut core::ffi::c_void))
                .map(hwnd_to_u64)
                .unwrap_or(0);
            let siblings = progman_child_sibling_order(progman);
            for (h, class, z) in &siblings {
                diag.push_str(&format!(
                    "  final sibling hwnd=0x{h:x} class={class} z={z}\n"
                ));
            }
            let visible = IsWindowVisible(hwnd).as_bool()
                || (host_hwnd != 0
                    && IsWindowVisible(HWND(host_hwnd as *mut core::ffi::c_void)).as_bool());
            ProgmanAttachVerifyInput {
                progman_hwnd: plan.parent_hwnd,
                our_hwnd: hwnd_to_u64(hwnd),
                our_parent: hwnd_to_u64(our_parent),
                our_parent_expected,
                defview_hwnd: plan.defview_hwnd,
                wallpaper_workerw_hwnd: plan.wallpaper_workerw_hwnd,
                rect_w,
                rect_h,
                visible,
                siblings_top_to_bottom: siblings.iter().map(|(h, _, z)| (*h, *z)).collect(),
                z_order_hwnd,
                z_order_parent: z_order_parent_hwnd,
            }
        }
    }

    fn verify_attach(
        hwnd: HWND,
        plan: &WallpaperAttachPlan,
        path: ProgmanAttachPath,
        host_hwnd: u64,
        diag: &mut String,
    ) -> Result<(), String> {
        let input = collect_verify_input(hwnd, plan, path, host_hwnd, diag);
        verify_progman_layered_state(&input).map_err(|e| e.to_string())
    }

    fn attach_progman_layered(
        hwnd: HWND,
        plan: WallpaperAttachPlan,
        snapshot: WallpaperRestore,
        diag: &mut String,
    ) -> Result<(ProgmanAttachPath, u64), String> {
        let parent = HWND(plan.parent_hwnd as *mut core::ffi::c_void);
        let insert_after = HWND(plan.z_insert_after_hwnd as *mut core::ffi::c_void);
        let (x, y, w, h) = child_screen_rect(parent, hwnd);

        set_redraw(hwnd, false, diag);
        force_clear_topmost(hwnd, diag);
        diag.push_str("attach path A: direct Progman child (no WS_EX_LAYERED)\n");
        let path_a_result = reparent_fullscreen(
            hwnd,
            parent,
            "Progman",
            Some(DesktopLayout::ProgmanLayered),
            &snapshot,
            diag,
            Some(insert_after),
            true,
            false,
        );
        if let Err(e) = path_a_result {
            diag.push_str(&format!("path A attach failed: {e} — trying path B\n"));
        } else if let Err(e) = finalize_progman_show(hwnd, insert_after, None, diag) {
            diag.push_str(&format!("path A finalize failed: {e} — trying path B\n"));
        } else {
            let verify = verify_attach(hwnd, &plan, ProgmanAttachPath::A, 0, diag);
            match verify {
                Ok(()) => {
                    diag.push_str("path A: success\n");
                    return Ok((ProgmanAttachPath::A, 0));
                }
                Err(e) => {
                    diag.push_str(&format!("path A verify failed: {e} — trying path B\n"));
                }
            }
        }

        let mut snap = snapshot.clone();
        rollback_attach(hwnd, &snap, diag);

        diag.push_str("attach path B: layered host + reparent content\n");
        set_redraw(hwnd, false, diag);
        let host = create_layered_host(parent, &plan, x, y, w, h, true, diag)?;
        let host_id = hwnd_to_u64(host);
        reparent_fullscreen(
            hwnd,
            host,
            "LyricGlassHost",
            Some(DesktopLayout::ProgmanLayered),
            &snapshot,
            diag,
            None,
            true,
            true,
        )?;
        if let Err(e) = finalize_progman_show(hwnd, insert_after, Some(host), diag) {
            snap.host_hwnd = host_id;
            rollback_attach(hwnd, &snap, diag);
            return Err(format!("path B finalize: {e}"));
        }
        let verify = verify_attach(hwnd, &plan, ProgmanAttachPath::B, host_id, diag);
        if verify.is_err() {
            snap.host_hwnd = host_id;
            rollback_attach(hwnd, &snap, diag);
            return Err(format!("path B verify: {}", verify.err().unwrap_or_default()));
        }
        diag.push_str("path B: success\n");
        Ok((ProgmanAttachPath::B, host_id))
    }

    pub fn attach_wallpaper(hwnd: HWND, trigger: &str) -> Result<(), String> {
        let mut diag = format!(
            "windows: {}\ntrigger: {trigger}\nstep: begin attach\nour_hwnd=0x{:x}\n",
            windows_build_label(),
            hwnd_to_u64(hwnd)
        );
        let progman_hwnd_early = progman();
        log_shell_defview(&mut diag, progman_hwnd_early);

        let mut guard = WALLPAPER_STATE.lock();
        if guard.attached {
            finish_attempt(trigger, "already_attached", true, None, diag);
            return Ok(());
        }

        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(hwnd, &mut rect);
            guard.overlay = OverlayGeometry {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(0) as u32,
                height: (rect.bottom - rect.top).max(0) as u32,
            };
        }
        guard.previous_style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) };
        guard.previous_ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
        let snapshot = guard.clone();
        drop(guard);

        if let Err(e) = spawn_desktop_workerw(&mut diag) {
            finish_attempt(trigger, "failure", false, Some(format!("spawn: {e}")), diag);
            return Err(e);
        }

        let progman_hwnd = progman_hwnd_early.or_else(progman);
        let mut layered_plan: Option<WallpaperAttachPlan> = None;
        if let Some(progman) = progman_hwnd {
            diag.push_str(&format!("progman_children:\n"));
            let children = enumerate_progman_children(progman, &mut diag);
            layered_plan = plan_progman_layered_attach(&children, hwnd_to_u64(progman));
            if let Some(ref plan) = layered_plan {
                diag.push_str(&format!(
                    "plan: ProgmanLayered parent=0x{:x} defview=0x{:x} wallpaper_ww=0x{:x} insert_after=0x{:x}\n",
                    plan.parent_hwnd,
                    plan.defview_hwnd,
                    plan.wallpaper_workerw_hwnd,
                    plan.z_insert_after_hwnd
                ));
            }
        }

        let host_result = wait_for_workerw(&mut diag);
        if let Err(ref e) = host_result {
            diag.push_str(&format!("find_workerw (legacy/top-level) failed: {e}\n"));
        }

        let mut attach_result: Result<(), String> = Err("no host".into());
        let mut chosen_path: Option<ProgmanAttachPath> = None;
        let mut host_hwnd: u64 = 0;

        if let Some(plan) = layered_plan {
            match attach_progman_layered(hwnd, plan, snapshot.clone(), &mut diag) {
                Ok((path, host)) => {
                    attach_result = Ok(());
                    chosen_path = Some(path);
                    host_hwnd = host;
                }
                Err(e) => attach_result = Err(e),
            }
        } else if let Ok((workerw, layout)) = host_result {
            diag.push_str(&format!("chosen_host=WorkerW 0x{:x}\n", hwnd_to_u64(workerw)));
            set_redraw(hwnd, false, &mut diag);
            attach_result = reparent_fullscreen(
                hwnd,
                workerw,
                "WorkerW",
                Some(layout),
                &snapshot,
                &mut diag,
                None,
                true,
                false,
            );
            if attach_result.is_ok() {
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                    );
                }
                set_redraw(hwnd, true, &mut diag);
            }
        }

        let mut guard = WALLPAPER_STATE.lock();
        match attach_result {
            Ok(()) => {
                guard.attached = true;
                guard.host_hwnd = host_hwnd;
                guard.attach_path = chosen_path;
                if let Some(ref p) = chosen_path {
                    diag.push_str(&format!("outcome: success path={p:?}\n"));
                } else {
                    diag.push_str("outcome: success legacy\n");
                }
                finish_attempt(trigger, "success", true, None, diag);
                Ok(())
            }
            Err(ref e) => {
                let mut snap = snapshot;
                snap.host_hwnd = host_hwnd;
                rollback_attach(hwnd, &snap, &mut diag);
                guard.attached = false;
                guard.host_hwnd = 0;
                guard.attach_path = None;
                diag.push_str(&format!("outcome: failure ({e}) — rolled back overlay\n"));
                finish_attempt(trigger, "failure", false, Some(e.clone()), diag);
                Err(e.clone())
            }
        }
    }

    pub fn detach_wallpaper(hwnd: HWND, trigger: &str) -> Result<(), String> {
        let mut guard = WALLPAPER_STATE.lock();
        if !guard.attached {
            return Ok(());
        }

        let mut trace = format!(
            "detach trigger={trigger} path={:?} host=0x{:x} restored_geometry={:?}\n",
            guard.attach_path,
            guard.host_hwnd,
            guard.overlay
        );
        let snap = guard.clone();
        guard.attached = false;
        guard.host_hwnd = 0;
        guard.attach_path = None;
        drop(guard);

        rollback_attach(hwnd, &snap, &mut trace);
        finish_attempt(trigger, "detached", false, None, trace);
        Ok(())
    }

    pub fn take_overlay_geometry() -> Option<OverlayGeometry> {
        let guard = WALLPAPER_STATE.lock();
        if guard.overlay.width > 0 && guard.overlay.height > 0 {
            Some(guard.overlay.clone())
        } else {
            None
        }
    }

    pub fn is_attached() -> bool {
        WALLPAPER_STATE.lock().attached
    }

    #[cfg(test)]
    mod attach_integration {
        use super::*;
        use crate::wallpaper_select::{
            verify_progman_layered_state, ProgmanAttachPath, ProgmanAttachVerifyInput,
            WallpaperAttachPlan,
        };

        unsafe extern "system" fn fake_wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> windows::Win32::Foundation::LRESULT {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        struct FakeDesktop {
            progman: HWND,
            defview: HWND,
            workerw: HWND,
            content: HWND,
        }

        impl Drop for FakeDesktop {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.content);
                    let _ = DestroyWindow(self.workerw);
                    let _ = DestroyWindow(self.defview);
                    let _ = DestroyWindow(self.progman);
                }
            }
        }

        fn create_fake_desktop() -> Result<FakeDesktop, String> {
            unsafe {
                for class in [
                    FAKE_PROGMAN_CLASS,
                    FAKE_DEFVIEW_CLASS,
                    FAKE_WORKERW_CLASS,
                    FAKE_CONTENT_CLASS,
                ] {
                    register_wnd_class(class, Some(fake_wnd_proc))?;
                }
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;
                use windows::core::PCWSTR;
                let hinst = GetModuleHandleW(None).unwrap_or_default();
                let make = |class: &str| -> Result<Vec<u16>, String> {
                    Ok(OsStr::new(class)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect())
                };
                let progman = CreateWindowExW(
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                    PCWSTR(make(FAKE_PROGMAN_CLASS)?.as_ptr()),
                    PCWSTR(make("FakeProgman")?.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x00CF0000),
                    50,
                    50,
                    640,
                    480,
                    None,
                    None,
                    Some(hinst.into()),
                    None,
                )
                .map_err(|e| format!("fake progman: {e}"))?;
                let defview = CreateWindowExW(
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                    PCWSTR(make(FAKE_DEFVIEW_CLASS)?.as_ptr()),
                    PCWSTR(make("defview")?.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(WS_CHILD | WS_VISIBLE),
                    0,
                    0,
                    640,
                    480,
                    Some(progman),
                    None,
                    Some(hinst.into()),
                    None,
                )
                .map_err(|e| format!("fake defview: {e}"))?;
                let workerw = CreateWindowExW(
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                    PCWSTR(make(FAKE_WORKERW_CLASS)?.as_ptr()),
                    PCWSTR(make("workerw")?.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(WS_CHILD | WS_VISIBLE),
                    0,
                    0,
                    640,
                    480,
                    Some(progman),
                    None,
                    Some(hinst.into()),
                    None,
                )
                .map_err(|e| format!("fake workerw: {e}"))?;
                let content = CreateWindowExW(
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE::default(),
                    PCWSTR(make(FAKE_CONTENT_CLASS)?.as_ptr()),
                    PCWSTR(make("content")?.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0x00CF0000),
                    80,
                    80,
                    400,
                    300,
                    None,
                    None,
                    Some(hinst.into()),
                    None,
                )
                .map_err(|e| format!("fake content: {e}"))?;
                Ok(FakeDesktop {
                    progman,
                    defview,
                    workerw,
                    content,
                })
            }
        }

        #[test]
        fn layered_host_creates_under_fake_progman() {
            let fake = create_fake_desktop().expect("fake desktop");
            let mut diag = String::new();
            let plan = WallpaperAttachPlan {
                layout: crate::wallpaper_select::DesktopLayout::ProgmanLayered,
                parent_hwnd: hwnd_to_u64(fake.progman),
                z_insert_after_hwnd: hwnd_to_u64(fake.defview),
                defview_hwnd: hwnd_to_u64(fake.defview),
                wallpaper_workerw_hwnd: hwnd_to_u64(fake.workerw),
            };
            let host = create_layered_host(fake.progman, &plan, 0, 0, 400, 300, false, &mut diag)
                .expect("host create");
            assert!(hwnd_valid(host));
            unsafe {
                let _ = DestroyWindow(host);
            }
        }

        #[test]
        fn fake_progman_path_a_visible_and_z_order() {
            let fake = create_fake_desktop().expect("fake desktop");
            let snapshot = WallpaperRestore::default();
            let mut diag = String::new();
            let plan = WallpaperAttachPlan {
                layout: crate::wallpaper_select::DesktopLayout::ProgmanLayered,
                parent_hwnd: hwnd_to_u64(fake.progman),
                z_insert_after_hwnd: hwnd_to_u64(fake.defview),
                defview_hwnd: hwnd_to_u64(fake.defview),
                wallpaper_workerw_hwnd: hwnd_to_u64(fake.workerw),
            };
            let (path, host_id) =
                attach_progman_layered(fake.content, plan.clone(), snapshot, &mut diag)
                    .expect("path A attach");
            assert_eq!(path, ProgmanAttachPath::A);
            assert_eq!(host_id, 0);
            let verify = verify_attach(
                fake.content,
                &plan,
                ProgmanAttachPath::A,
                0,
                &mut diag,
            )
            .expect("verify");
            let _ = verify;
            let input = collect_verify_input(
                fake.content,
                &plan,
                ProgmanAttachPath::A,
                0,
                &mut diag,
            );
            verify_progman_layered_state(&input).expect("z-order");
            assert!(input.visible);
            unsafe {
                let _ = SetParent(fake.content, None);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn attach_wallpaper(_hwnd: (), _trigger: &str) -> Result<(), String> {
        Err("Wallpaper attach is Windows-only".into())
    }

    pub fn detach_wallpaper(_hwnd: (), _trigger: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn take_overlay_geometry() -> Option<OverlayGeometry> {
        None
    }

    pub fn is_attached() -> bool {
        false
    }
}

#[cfg(windows)]
pub use imp::{attach_wallpaper, detach_wallpaper, is_attached, take_overlay_geometry};

#[cfg(not(windows))]
pub use imp::{attach_wallpaper, detach_wallpaper, is_attached, take_overlay_geometry};

#[cfg(test)]
mod tests {
    use super::OverlayGeometry;

    #[test]
    fn overlay_geometry_default_is_zero() {
        let g = OverlayGeometry::default();
        assert_eq!(g.width, 0);
        assert_eq!(g.height, 0);
    }
}