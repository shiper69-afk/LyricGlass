mod clock;
mod media;
mod startup;
mod wallpaper;

use media::{now_unix_ms, smoothed_now, MediaState, SharedMediaState, start_media_watcher};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct AppFlags {
    click_through: bool,
}

type SharedFlags = Arc<Mutex<AppFlags>>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiState {
    click_through: bool,
    wallpaper_mode: bool,
}

fn current_ui_state(flags: &SharedFlags) -> UiState {
    UiState {
        click_through: flags.lock().click_through,
        wallpaper_mode: wallpaper::is_attached(),
    }
}

#[tauri::command]
fn get_media_state(state: tauri::State<SharedMediaState>) -> MediaState {
    state.read().clone()
}

#[tauri::command]
fn get_smoothed_position(state: tauri::State<SharedMediaState>) -> u64 {
    let snap = state.read().clone();
    smoothed_now(&snap, now_unix_ms())
}

#[tauri::command]
fn get_ui_state(flags: tauri::State<SharedFlags>) -> UiState {
    current_ui_state(flags.inner())
}

#[tauri::command]
fn get_startup_log_path() -> String {
    startup::log_path_display()
}

fn set_click_through(window: &WebviewWindow, flags: &SharedFlags, enabled: bool) -> Result<(), String> {
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|e| e.to_string())?;
    flags.lock().click_through = enabled;
    Ok(())
}

fn set_wallpaper_mode_inner(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;

        if enabled {
            wallpaper::attach_wallpaper(hwnd)?;
            let _ = window.set_always_on_top(false);
        } else {
            wallpaper::detach_wallpaper(hwnd)?;
            let _ = window.set_always_on_top(true);
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (window, enabled);
        return Err("Wallpaper mode is Windows-only".into());
    }

    Ok(())
}

#[tauri::command]
fn toggle_click_through(
    window: WebviewWindow,
    flags: tauri::State<SharedFlags>,
    app: AppHandle,
) -> Result<bool, String> {
    let next = !flags.lock().click_through;
    set_click_through(&window, flags.inner(), next)?;
    let _ = app.emit("ui-state", current_ui_state(flags.inner()));
    Ok(next)
}

#[tauri::command]
fn set_wallpaper_mode(
    window: WebviewWindow,
    enabled: bool,
    flags: tauri::State<SharedFlags>,
    app: AppHandle,
) -> Result<(), String> {
    set_wallpaper_mode_inner(&window, enabled)?;
    let _ = app.emit("ui-state", current_ui_state(flags.inner()));
    Ok(())
}

#[tauri::command]
fn toggle_wallpaper_mode(
    window: WebviewWindow,
    flags: tauri::State<SharedFlags>,
    app: AppHandle,
) -> Result<bool, String> {
    let next = !wallpaper::is_attached();
    set_wallpaper_mode_inner(&window, next)?;
    let _ = app.emit("ui-state", current_ui_state(flags.inner()));
    Ok(next)
}

fn register_shortcuts(app: &AppHandle) {
    register_one_shortcut(app, "Ctrl+Alt+C", true);
    register_one_shortcut(app, "Ctrl+Alt+W", false);
}

fn register_one_shortcut(app: &AppHandle, shortcut_str: &str, click: bool) {
    let parsed: Shortcut = match shortcut_str.parse() {
        Ok(s) => s,
        Err(e) => {
            startup::step(&format!("hotkey parse failed ({shortcut_str}): {e}"));
            return;
        }
    };

    let app_handle = app.clone();
    if let Err(e) = app.global_shortcut().on_shortcut(parsed, move |_app, _shortcut, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        if let Some(window) = app_handle.get_webview_window("main") {
            let flags = app_handle.state::<SharedFlags>();
            if click {
                let next = !flags.lock().click_through;
                if set_click_through(&window, flags.inner(), next).is_ok() {
                    let _ = app_handle.emit("ui-state", current_ui_state(flags.inner()));
                }
            } else if set_wallpaper_mode_inner(&window, !wallpaper::is_attached()).is_ok() {
                let _ = app_handle.emit("ui-state", current_ui_state(flags.inner()));
            }
        }
    }) {
        startup::step(&format!("hotkey handler failed ({shortcut_str}): {e}"));
    } else {
        startup::step(&format!("hotkey ready: {shortcut_str}"));
    }
}

fn build_tray(app: &AppHandle) {
    let toggle_click = match MenuItem::with_id(app, "toggle_click", "Toggle click-through", true, None::<&str>) {
        Ok(i) => i,
        Err(e) => {
            startup::step(&format!("tray menu item failed: {e}"));
            return;
        }
    };
    let toggle_wallpaper = match MenuItem::with_id(app, "toggle_wallpaper", "Toggle wallpaper mode", true, None::<&str>) {
        Ok(i) => i,
        Err(e) => {
            startup::step(&format!("tray menu item failed: {e}"));
            return;
        }
    };
    let quit = match PredefinedMenuItem::quit(app, None) {
        Ok(i) => i,
        Err(e) => {
            startup::step(&format!("tray quit item failed: {e}"));
            return;
        }
    };
    let menu = match Menu::with_items(app, &[&toggle_click, &toggle_wallpaper, &quit]) {
        Ok(m) => m,
        Err(e) => {
            startup::step(&format!("tray menu failed: {e}"));
            return;
        }
    };

    let Some(icon) = app.default_window_icon().cloned() else {
        startup::step("tray skipped: no window icon available");
        return;
    };

    if let Err(e) = TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .tooltip("LyricGlass Phase 0 spike")
        .on_menu_event({
            let app_handle = app.clone();
            move |app, event| match event.id.as_ref() {
                "toggle_click" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let flags = app.state::<SharedFlags>();
                        let next = !flags.lock().click_through;
                        if set_click_through(&window, flags.inner(), next).is_ok() {
                            let _ = app_handle.emit("ui-state", current_ui_state(flags.inner()));
                        }
                    }
                }
                "toggle_wallpaper" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let flags = app.state::<SharedFlags>();
                        if set_wallpaper_mode_inner(&window, !wallpaper::is_attached()).is_ok() {
                            let _ = app_handle.emit("ui-state", current_ui_state(flags.inner()));
                        }
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)
    {
        startup::step(&format!("tray build failed: {e}"));
    } else {
        startup::step("tray ready");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    startup::install_panic_hook();
    startup::init_logging();
    startup::step("LyricGlass run() begin");

    let media_state: SharedMediaState = Arc::new(parking_lot::RwLock::new(MediaState::default()));
    let flags: SharedFlags = Arc::new(Mutex::new(AppFlags {
        click_through: false,
    }));

    startup::step("creating tauri builder");
    startup::step("entering tauri event loop");
    let run_result = tauri::Builder::default()
        .plugin({
            startup::step("loading global-shortcut plugin");
            tauri_plugin_global_shortcut::Builder::new().build()
        })
        .manage(media_state.clone())
        .manage(flags.clone())
        .invoke_handler(tauri::generate_handler![
            get_media_state,
            get_smoothed_position,
            get_ui_state,
            get_startup_log_path,
            toggle_click_through,
            set_wallpaper_mode,
            toggle_wallpaper_mode,
        ])
        .setup({
            let flags = flags.clone();
            move |app| {
                startup::step("tauri setup: window shell");
                startup::step("tauri setup: building tray (optional)");
                build_tray(app.handle());
                startup::step("tauri setup: registering hotkeys (optional)");
                register_shortcuts(app.handle());
                startup::step("tauri setup: starting media watcher (optional)");
                start_media_watcher(app.handle().clone(), media_state.clone());
                startup::step("tauri setup: emitting initial UI state");
                let _ = app.handle().emit("ui-state", current_ui_state(&flags));
                startup::step("tauri setup complete — WebView2 loading");
                Ok(())
            }
        })
        .run(tauri::generate_context!());

    match run_result {
        Ok(()) => startup::step("tauri exited normally"),
        Err(e) => startup::log_fatal_error("tauri run", e),
    }
}