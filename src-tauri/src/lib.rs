mod clock;
mod lyrics;
mod media;
mod startup;
mod wallpaper;

use lyrics::{create_service, LyricsResponse, SharedLyricsService};
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
    always_on_top: bool,
    debug_visible: bool,
}

type SharedFlags = Arc<Mutex<AppFlags>>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UiState {
    always_on_top: bool,
    wallpaper_mode: bool,
    debug_visible: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceState {
    overlay_opacity: f64,
    karaoke_ball: bool,
}

fn apply_topmost(window: &WebviewWindow, flags: &AppFlags) {
    let on_top = flags.always_on_top && !wallpaper::is_attached();
    let _ = window.set_always_on_top(on_top);
}

fn ui_state_from_flags(flags: &AppFlags) -> UiState {
    UiState {
        always_on_top: flags.always_on_top,
        wallpaper_mode: wallpaper::is_attached(),
        debug_visible: flags.debug_visible,
    }
}

fn current_ui_state(flags: &SharedFlags) -> UiState {
    ui_state_from_flags(&flags.lock())
}

fn toggle_debug_visible(app: &AppHandle, flags: &SharedFlags, source: &str) {
    startup::step(&format!("diag toggle ({source}): entered"));
    let ui = {
        startup::step(&format!("diag toggle ({source}): acquiring flags lock"));
        let mut guard = flags.lock();
        startup::step(&format!("diag toggle ({source}): lock acquired"));
        guard.debug_visible = !guard.debug_visible;
        let ui = ui_state_from_flags(&guard);
        startup::step(&format!(
            "diag toggle ({source}): visible={}",
            guard.debug_visible
        ));
        ui
    };
    startup::step(&format!("diag toggle ({source}): lock released"));
    startup::step(&format!("diag toggle ({source}): before ui-state emit"));
    let _ = app.emit("ui-state", ui);
    startup::step(&format!("diag toggle ({source}): after ui-state emit"));
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

#[tauri::command]
fn fetch_lyrics(
    service: tauri::State<SharedLyricsService>,
    artist: String,
    track: String,
    album: String,
    duration_ms: u64,
) -> LyricsResponse {
    service.fetch(&artist, &track, &album, duration_ms)
}

#[tauri::command]
fn get_sync_offset(
    service: tauri::State<SharedLyricsService>,
    artist: String,
    track: String,
    album: String,
    duration_ms: u64,
) -> i64 {
    service.get_sync_offset(&artist, &track, &album, duration_ms)
}

#[tauri::command]
fn set_sync_offset(
    service: tauri::State<SharedLyricsService>,
    artist: String,
    track: String,
    album: String,
    duration_ms: u64,
    offset_ms: i64,
    app: AppHandle,
) -> Result<i64, String> {
    let total = service.set_sync_offset(&artist, &track, &album, duration_ms, offset_ms)?;
    let _ = app.emit("sync-offset-absolute", total);
    let _ = app.emit("seek-tick", ());
    Ok(total)
}

#[tauri::command]
fn get_appearance(service: tauri::State<SharedLyricsService>) -> AppearanceState {
    AppearanceState {
        overlay_opacity: service.get_overlay_opacity(),
        karaoke_ball: service.get_karaoke_ball(),
    }
}

#[tauri::command]
fn set_karaoke_ball(
    service: tauri::State<SharedLyricsService>,
    enabled: bool,
    app: AppHandle,
) -> Result<bool, String> {
    let value = service.set_karaoke_ball(enabled)?;
    let _ = app.emit(
        "appearance-state",
        AppearanceState {
            overlay_opacity: service.get_overlay_opacity(),
            karaoke_ball: value,
        },
    );
    Ok(value)
}

#[tauri::command]
fn set_overlay_opacity(
    service: tauri::State<SharedLyricsService>,
    opacity: f64,
    app: AppHandle,
) -> Result<f64, String> {
    let value = service.set_overlay_opacity(opacity)?;
    let _ = app.emit(
        "appearance-state",
        AppearanceState {
            overlay_opacity: value,
            karaoke_ball: service.get_karaoke_ball(),
        },
    );
    Ok(value)
}

#[tauri::command]
fn log_diagnostics(block: String) {
    startup::log_diagnostic(&block);
}

#[tauri::command]
fn log_sync_mark(block: String) {
    startup::log_diagnostic(&format!("SYNC MARK\n{block}"));
}

#[tauri::command]
fn toggle_debug(
    flags: tauri::State<SharedFlags>,
    app: AppHandle,
) -> Result<bool, String> {
    toggle_debug_visible(&app, flags.inner(), "command");
    Ok(flags.lock().debug_visible)
}

fn set_wallpaper_mode_inner(
    window: &WebviewWindow,
    flags: &SharedFlags,
    enabled: bool,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;

        if enabled {
            wallpaper::attach_wallpaper(hwnd)?;
        } else {
            wallpaper::detach_wallpaper(hwnd)?;
        }
        apply_topmost(window, &flags.lock());
    }

    #[cfg(not(windows))]
    {
        let _ = (window, flags, enabled);
        return Err("Wallpaper mode is Windows-only".into());
    }

    Ok(())
}

#[tauri::command]
fn toggle_always_on_top(
    window: WebviewWindow,
    flags: tauri::State<SharedFlags>,
    service: tauri::State<SharedLyricsService>,
    app: AppHandle,
) -> Result<bool, String> {
    let next = {
        let mut guard = flags.lock();
        guard.always_on_top = !guard.always_on_top;
        service.set_always_on_top(guard.always_on_top)?;
        apply_topmost(&window, &guard);
        guard.always_on_top
    };
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
    set_wallpaper_mode_inner(&window, flags.inner(), enabled)?;
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
    set_wallpaper_mode_inner(&window, flags.inner(), next)?;
    let _ = app.emit("ui-state", current_ui_state(flags.inner()));
    Ok(next)
}

enum ShortcutAction {
    ToggleWallpaper,
    SyncOffset(i64),
    MarkSync,
}

fn register_shortcuts(app: &AppHandle) {
    register_one_shortcut(app, "Ctrl+Alt+W", ShortcutAction::ToggleWallpaper);
    register_one_shortcut(app, "Ctrl+Alt+[", ShortcutAction::SyncOffset(-100));
    register_one_shortcut(app, "Ctrl+Alt+]", ShortcutAction::SyncOffset(100));
    register_one_shortcut(app, "Ctrl+Alt+M", ShortcutAction::MarkSync);
}

fn register_one_shortcut(app: &AppHandle, shortcut_str: &str, action: ShortcutAction) {
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
        match action {
            ShortcutAction::MarkSync => {
                let _ = app_handle.emit("sync-mark-request", ());
            }
            ShortcutAction::SyncOffset(delta) => {
                let media = app_handle.state::<SharedMediaState>().read().clone();
                if media.artist.trim().is_empty() && media.title.trim().is_empty() {
                    return;
                }
                let service = app_handle.state::<SharedLyricsService>();
                match service.add_sync_offset(
                    &media.artist,
                    &media.title,
                    &media.album,
                    media.duration_ms,
                    delta,
                ) {
                    Ok(total) => {
                        let _ = app_handle.emit("sync-offset-absolute", total);
                        let _ = app_handle.emit("seek-tick", ());
                    }
                    Err(e) => startup::step(&format!("sync offset save failed: {e}")),
                }
            }
            ShortcutAction::ToggleWallpaper => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let flags = app_handle.state::<SharedFlags>();
                    if set_wallpaper_mode_inner(
                        &window,
                        flags.inner(),
                        !wallpaper::is_attached(),
                    )
                    .is_ok()
                    {
                        let _ = app_handle.emit("ui-state", current_ui_state(flags.inner()));
                    }
                }
            }
        }
    }) {
        startup::step(&format!("hotkey handler failed ({shortcut_str}): {e}"));
    } else {
        startup::step(&format!("hotkey ready: {shortcut_str}"));
    }
}

fn build_tray(app: &AppHandle) {
    let toggle_wallpaper = match MenuItem::with_id(app, "toggle_wallpaper", "Toggle wallpaper mode", true, None::<&str>) {
        Ok(i) => i,
        Err(e) => {
            startup::step(&format!("tray menu item failed: {e}"));
            return;
        }
    };
    let toggle_debug = match MenuItem::with_id(app, "toggle_debug", "Show diagnostics", true, None::<&str>) {
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
    let menu = match Menu::with_items(app, &[&toggle_wallpaper, &toggle_debug, &quit]) {
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
        .tooltip("LyricGlass")
        .on_menu_event({
            let app_handle = app.clone();
            move |app, event| match event.id.as_ref() {
                "toggle_wallpaper" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let flags = app.state::<SharedFlags>();
                        if set_wallpaper_mode_inner(
                            &window,
                            flags.inner(),
                            !wallpaper::is_attached(),
                        )
                        .is_ok()
                        {
                            let _ = app_handle.emit("ui-state", current_ui_state(flags.inner()));
                        }
                    }
                }
                "toggle_debug" => {
                    let flags = app.state::<SharedFlags>();
                    let app_handle = app_handle.clone();
                    let flags = flags.inner().clone();
                    std::thread::spawn(move || {
                        startup::step("tray toggle_debug: worker spawned");
                        let app = app_handle.clone();
                        let _ = app_handle.run_on_main_thread(move || {
                            toggle_debug_visible(&app, &flags, "tray");
                        });
                    });
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
    let lyrics_service = create_service();
    let flags: SharedFlags = Arc::new(Mutex::new(AppFlags {
        always_on_top: true,
        debug_visible: false,
    }));

    startup::step("creating tauri builder");
    startup::step("entering tauri event loop");
    let run_result = tauri::Builder::default()
        .plugin({
            startup::step("loading global-shortcut plugin");
            tauri_plugin_global_shortcut::Builder::new().build()
        })
        .manage(media_state.clone())
        .manage(lyrics_service)
        .manage(flags.clone())
        .invoke_handler(tauri::generate_handler![
            get_media_state,
            get_smoothed_position,
            get_ui_state,
            get_startup_log_path,
            fetch_lyrics,
            get_sync_offset,
            set_sync_offset,
            get_appearance,
            set_overlay_opacity,
            set_karaoke_ball,
            log_diagnostics,
            log_sync_mark,
            toggle_debug,
            toggle_always_on_top,
            set_wallpaper_mode,
            toggle_wallpaper_mode,
        ])
        .setup({
            let flags = flags.clone();
            move |app| {
                startup::step("tauri setup: window shell");
                let service = app.state::<SharedLyricsService>();
                {
                    let mut guard = flags.lock();
                    guard.always_on_top = service.get_always_on_top();
                }
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_ignore_cursor_events(false);
                    apply_topmost(&window, &flags.lock());
                    startup::step("startup: click-through cleared, always-on-top applied");
                }
                startup::step("tauri setup: building tray (optional)");
                build_tray(app.handle());
                startup::step("tauri setup: registering hotkeys (optional)");
                register_shortcuts(app.handle());
                startup::step("tauri setup: starting media watcher (optional)");
                start_media_watcher(app.handle().clone(), media_state.clone());
                startup::step("tauri setup: emitting initial UI state");
                let _ = app.handle().emit("ui-state", current_ui_state(&flags));
                let appearance = app.state::<SharedLyricsService>();
                let _ = app.handle().emit(
                    "appearance-state",
                    AppearanceState {
                        overlay_opacity: appearance.get_overlay_opacity(),
                        karaoke_ball: appearance.get_karaoke_ball(),
                    },
                );
                startup::step("tauri setup complete — WebView2 loading");
                if std::env::var("LYRICGLASS_SMOKE_DIAG").is_ok() {
                    let app = app.handle().clone();
                    let flags = flags.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        startup::step("smoke: scheduling diagnostics toggle");
                        let app_cb = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            toggle_debug_visible(&app_cb, &flags, "smoke");
                        });
                    });
                }
                Ok(())
            }
        })
        .run(tauri::generate_context!());

    match run_result {
        Ok(()) => startup::step("tauri exited normally"),
        Err(e) => startup::log_fatal_error("tauri run", e),
    }
}