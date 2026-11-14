mod clock;
mod media;
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

fn register_shortcuts(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let click_shortcut: Shortcut = "Ctrl+Alt+C".parse()?;
    let wallpaper_shortcut: Shortcut = "Ctrl+Alt+W".parse()?;

    let app_click = app.clone();
    app.global_shortcut().on_shortcut(click_shortcut, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            if let Some(window) = app_click.get_webview_window("main") {
                let flags = app_click.state::<SharedFlags>();
                let next = !flags.lock().click_through;
                if set_click_through(&window, flags.inner(), next).is_ok() {
                    let _ = app_click.emit("ui-state", current_ui_state(flags.inner()));
                }
            }
        }
    })?;

    let app_wallpaper = app.clone();
    app.global_shortcut().on_shortcut(wallpaper_shortcut, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            if let Some(window) = app_wallpaper.get_webview_window("main") {
                let flags = app_wallpaper.state::<SharedFlags>();
                if set_wallpaper_mode_inner(&window, !wallpaper::is_attached()).is_ok() {
                    let _ = app_wallpaper.emit("ui-state", current_ui_state(flags.inner()));
                }
            }
        }
    })?;

    app.global_shortcut().register(click_shortcut)?;
    app.global_shortcut().register(wallpaper_shortcut)?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_click = MenuItem::with_id(app, "toggle_click", "Toggle click-through", true, None::<&str>)?;
    let toggle_wallpaper =
        MenuItem::with_id(app, "toggle_wallpaper", "Toggle wallpaper mode", true, None::<&str>)?;
    let quit = PredefinedMenuItem::quit(app, None)?;
    let menu = Menu::with_items(app, &[&toggle_click, &toggle_wallpaper, &quit])?;

    let _tray = TrayIconBuilder::with_id("main")
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
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let media_state: SharedMediaState = Arc::new(parking_lot::RwLock::new(MediaState::default()));
    let flags: SharedFlags = Arc::new(Mutex::new(AppFlags {
        click_through: false,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(media_state.clone())
        .manage(flags.clone())
        .invoke_handler(tauri::generate_handler![
            get_media_state,
            get_smoothed_position,
            get_ui_state,
            toggle_click_through,
            set_wallpaper_mode,
            toggle_wallpaper_mode,
        ])
        .setup(move |app| {
            build_tray(app.handle())?;
            register_shortcuts(app.handle())?;
            start_media_watcher(app.handle().clone(), media_state.clone());
            let _ = app.handle().emit("ui-state", current_ui_state(&flags));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}