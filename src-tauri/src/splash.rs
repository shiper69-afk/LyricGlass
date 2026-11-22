use tauri::{AppHandle, Manager};

use crate::startup;

pub fn should_skip_splash() -> bool {
    std::env::var("LYRICGLASS_SKIP_SPLASH").is_ok()
        || std::env::var("LYRICGLASS_SMOKE_DIAG").is_ok()
        || std::env::var("LYRICGLASS_SMOKE_SINGLE_INSTANCE").is_ok()
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

pub fn initialize_startup_windows(app: &AppHandle) {
    if should_skip_splash() {
        startup::step("splash: skipped (smoke or LYRICGLASS_SKIP_SPLASH)");
        finish_startup_splash(app);
        return;
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.show();
        let _ = splash.set_focus();
    }
    startup::step("splash: showing startup animation");
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
        assert!(!should_skip_splash());
    }
}