use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::startup;

pub fn should_skip_splash() -> bool {
    std::env::var("LYRICGLASS_SKIP_SPLASH").is_ok()
        || std::env::var("LYRICGLASS_SMOKE_DIAG").is_ok()
        || std::env::var("LYRICGLASS_SMOKE_SINGLE_INSTANCE").is_ok()
        || std::env::var("CI").ok().as_deref() == Some("true")
}

pub fn close_splash_window(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.hide();
        let _ = splash.close();
    }
}

pub fn finish_startup_splash(app: &AppHandle) {
    close_splash_window(app);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
    startup::step("splash: handoff complete — main window shown");
}

/// If the splash WebView never calls `complete_startup_splash`, show main anyway.
pub fn arm_splash_watchdog(app: &AppHandle) {
    if should_skip_splash() {
        return;
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(4000));
        let watchdog = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if watchdog.get_webview_window("splash").is_some() {
                startup::step("splash: watchdog handoff (timeout)");
                finish_startup_splash(&watchdog);
            }
        });
    });
}

fn ensure_splash_window(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("splash").is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash.html".into()))
        .title("LyricGlass")
        .inner_size(400.0, 560.0)
        .decorations(false)
        .transparent(false)
        .always_on_top(true)
        .resizable(false)
        .shadow(false)
        .skip_taskbar(true)
        .center()
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn initialize_startup_windows(app: &AppHandle) {
    if should_skip_splash() {
        startup::step("splash: skipped (smoke, CI, or LYRICGLASS_SKIP_SPLASH)");
        finish_startup_splash(app);
        return;
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    match ensure_splash_window(app) {
        Ok(()) => {
            if let Some(splash) = app.get_webview_window("splash") {
                let _ = splash.show();
                let _ = splash.set_focus();
            }
            startup::step("splash: showing startup animation");
        }
        Err(e) => {
            startup::step(&format!("splash: create failed ({e}) — handoff to main"));
            finish_startup_splash(app);
        }
    }
}

#[tauri::command]
pub fn complete_startup_splash(app: AppHandle) -> Result<(), String> {
    finish_startup_splash(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::should_skip_splash;

    #[test]
    fn skip_splash_is_false_without_env() {
        std::env::remove_var("LYRICGLASS_SKIP_SPLASH");
        std::env::remove_var("LYRICGLASS_SMOKE_DIAG");
        std::env::remove_var("LYRICGLASS_SMOKE_SINGLE_INSTANCE");
        std::env::remove_var("CI");
        assert!(!should_skip_splash());
    }

    #[test]
    fn skip_splash_when_ci_env_set() {
        std::env::set_var("CI", "true");
        assert!(should_skip_splash());
        std::env::remove_var("CI");
    }
}