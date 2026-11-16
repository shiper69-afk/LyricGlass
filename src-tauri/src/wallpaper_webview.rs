//! WebView2 controller sync after Progman reparent (brief step 7).

#[cfg(windows)]
pub fn sync_webview_controller(window: &tauri::WebviewWindow, diag: &mut String) {
    let Ok(hwnd_raw) = window.hwnd() else {
        diag.push_str("sync_webview: hwnd unavailable\n");
        return;
    };
    let host_usize = hwnd_raw.0 as usize;

    let _ = window.with_webview(|platform| {
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

            let host = HWND(host_usize as _);
            let mut client = windows::Win32::Foundation::RECT::default();
            if GetClientRect(host, &mut client).is_err() {
                diag.push_str("sync_webview: GetClientRect failed\n");
                return;
            }

            let controller = platform.controller();

            if let Err(e) = controller.SetBounds(client) {
                diag.push_str(&format!("sync_webview: SetBounds err={e:?}\n"));
            }
            if let Err(e) = controller.SetIsVisible(true) {
                diag.push_str(&format!("sync_webview: SetIsVisible err={e:?}\n"));
            }
            if let Err(e) = controller.NotifyParentWindowPositionChanged() {
                diag.push_str(&format!("sync_webview: NotifyParent err={e:?}\n"));
            } else {
                diag.push_str("sync_webview: controller SetBounds/Visible/NotifyParent ok\n");
            }
        }
    });
}

#[cfg(not(windows))]
pub fn sync_webview_controller(_window: &tauri::WebviewWindow, _diag: &mut String) {}