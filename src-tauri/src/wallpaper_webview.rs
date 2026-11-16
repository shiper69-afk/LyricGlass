//! WebView2 controller sync after Progman reparent (brief step 7).

#[cfg(windows)]
pub fn sync_webview_controller(window: &tauri::WebviewWindow, diag: &mut String) {
    let host_usize = match window.hwnd() {
        Ok(h) => h.0,
        Err(_) => {
            diag.push_str("sync_webview: hwnd unavailable\n");
            return;
        }
    };

    let trace = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
    let trace_cb = trace.clone();
    let result = window.with_webview(move |platform| {
        let mut local = trace_cb.lock();
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

            let host = HWND(host_usize);
            let mut client = windows::Win32::Foundation::RECT::default();
            if GetClientRect(host, &mut client).is_err() {
                local.push_str("sync_webview: GetClientRect failed\n");
                return;
            }

            let controller = platform.controller();

            if let Err(e) = controller.SetBounds(client) {
                local.push_str(&format!("sync_webview: SetBounds err={e:?}\n"));
            }
            if let Err(e) = controller.SetIsVisible(true) {
                local.push_str(&format!("sync_webview: SetIsVisible err={e:?}\n"));
            }
            if let Err(e) = controller.NotifyParentWindowPositionChanged() {
                local.push_str(&format!("sync_webview: NotifyParent err={e:?}\n"));
            } else {
                local.push_str("sync_webview: controller SetBounds/Visible/NotifyParent ok\n");
            }
        }
    });

    if let Err(e) = result {
        diag.push_str(&format!("sync_webview: with_webview err={e}\n"));
    }
    diag.push_str(&trace.lock());
}

#[cfg(not(windows))]
pub fn sync_webview_controller(_window: &tauri::WebviewWindow, _diag: &mut String) {}