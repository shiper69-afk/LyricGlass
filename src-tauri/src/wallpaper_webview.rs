//! WebView2 controller sync after Progman reparent (brief step 7).

#[cfg(windows)]
pub fn sync_webview_controller(window: &tauri::WebviewWindow, diag: &mut String) {
    let host_usize = match window.hwnd() {
        Ok(h) => h.0 as usize,
        Err(_) => {
            diag.push_str("sync_webview: hwnd unavailable\n");
            return;
        }
    };

    let trace = window.with_webview(move |platform| {
        let mut local = String::new();
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

            let host = HWND(host_usize as _);
            let mut client = windows::Win32::Foundation::RECT::default();
            if GetClientRect(host, &mut client).is_err() {
                local.push_str("sync_webview: GetClientRect failed\n");
                return local;
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
        local
    });

    match trace {
        Ok(mut local) => diag.append(&mut local),
        Err(e) => diag.push_str(&format!("sync_webview: with_webview err={e}\n")),
    }
}

#[cfg(not(windows))]
pub fn sync_webview_controller(_window: &tauri::WebviewWindow, _diag: &mut String) {}