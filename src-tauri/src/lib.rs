mod app_version;
mod clock;
mod lyrics;
mod media;
mod startup;
mod window_focus;
mod desktop_wallpaper;
mod wallpaper;
mod wallpaper_clickthrough;
mod wallpaper_counters;
mod wallpaper_fallback;
mod wallpaper_hooks;
mod wallpaper_placement;
mod wallpaper_log;
mod wallpaper_select;
mod wallpaper_subclass;
mod wallpaper_webview;
#[cfg(windows)]
mod wallpaper_selftest;

use lyrics::{create_service, LyricsResponse, SharedLyricsService};
#[cfg(windows)]
use wallpaper_placement::{restore_main_placement, save_main_placement};
use media::{now_unix_ms, smoothed_now, MediaState, SharedMediaState, start_media_watcher};
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, WebviewWindow,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    wallpaper_error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WallpaperStatus {
    attached: bool,
    attempt_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_timestamp_unix_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_trigger: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_outcome: Option<String>,
    last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_diagnostics: Option<String>,
    log_file_path: String,
}

#[cfg(windows)]
pub fn run_wallpaper_selftest() -> i32 {
    wallpaper_selftest::run()
}

#[cfg(not(windows))]
pub fn run_wallpaper_selftest() -> i32 {
    eprintln!("--wallpaper-selftest requires Windows");
    2
}

fn build_wallpaper_status() -> WallpaperStatus {
    let attached = wallpaper::is_attached();
    let log_path = wallpaper_log::log_file_path();
    let log_file_path = log_path.to_string_lossy().into_owned();
    let attempt_count = wallpaper_log::attempt_count();
    match wallpaper_log::last_attempt() {
        Some(a) => WallpaperStatus {
            attached,
            attempt_count,
            last_timestamp_unix_ms: Some(a.timestamp_unix_ms),
            last_trigger: Some(a.trigger),
            last_outcome: Some(a.outcome),
            last_error: a.error,
            last_diagnostics: Some(a.trace),
            log_file_path,
        },
        None => WallpaperStatus {
            attached,
            attempt_count,
            last_timestamp_unix_ms: None,
            last_trigger: None,
            last_outcome: None,
            last_error: None,
            last_diagnostics: None,
            log_file_path,
        },
    }
}

fn emit_wallpaper_status(app: &AppHandle) {
    let status = build_wallpaper_status();
    let _ = app.emit("wallpaper-status", status.clone());
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppearanceState {
    overlay_opacity: f64,
    karaoke_ball: bool,
    karaoke_ball_color: String,
    sung_highlight_color: String,
    karaoke_bouncer: String,
    lyrics_font: String,
}

fn appearance_from_service(service: &SharedLyricsService) -> AppearanceState {
    AppearanceState {
        overlay_opacity: service.get_overlay_opacity(),
        karaoke_ball: service.get_karaoke_ball(),
        karaoke_ball_color: service.get_karaoke_ball_color(),
        sung_highlight_color: service.get_sung_highlight_color(),
        karaoke_bouncer: service.get_karaoke_bouncer(),
        lyrics_font: service.get_lyrics_font(),
    }
}

fn emit_appearance(service: &SharedLyricsService, app: &AppHandle) {
    let _ = app.emit("appearance-state", appearance_from_service(service));
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
        wallpaper_error: wallpaper::last_attach_error(),
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
    appearance_from_service(&service)
}

#[tauri::command]
fn set_karaoke_ball(
    service: tauri::State<SharedLyricsService>,
    enabled: bool,
    app: AppHandle,
) -> Result<bool, String> {
    let value = service.set_karaoke_ball(enabled)?;
    emit_appearance(&service, &app);
    Ok(value)
}

#[tauri::command]
fn set_overlay_opacity(
    service: tauri::State<SharedLyricsService>,
    opacity: f64,
    app: AppHandle,
) -> Result<f64, String> {
    let value = service.set_overlay_opacity(opacity)?;
    emit_appearance(&service, &app);
    Ok(value)
}

#[tauri::command]
fn set_karaoke_ball_color(
    service: tauri::State<SharedLyricsService>,
    color: String,
    app: AppHandle,
) -> Result<String, String> {
    let value = service.set_karaoke_ball_color(&color)?;
    emit_appearance(&service, &app);
    Ok(value)
}

#[tauri::command]
fn set_sung_highlight_color(
    service: tauri::State<SharedLyricsService>,
    color: String,
    app: AppHandle,
) -> Result<String, String> {
    let value = service.set_sung_highlight_color(&color)?;
    emit_appearance(&service, &app);
    Ok(value)
}

#[tauri::command]
fn reset_karaoke_colors(
    service: tauri::State<SharedLyricsService>,
    app: AppHandle,
) -> Result<(), String> {
    service.reset_karaoke_colors()?;
    emit_appearance(&service, &app);
    Ok(())
}

#[tauri::command]
fn set_karaoke_bouncer(
    service: tauri::State<SharedLyricsService>,
    bouncer_id: String,
    app: AppHandle,
) -> Result<String, String> {
    let value = service.set_karaoke_bouncer(&bouncer_id)?;
    emit_appearance(&service, &app);
    Ok(value)
}

#[tauri::command]
fn set_lyrics_font(
    service: tauri::State<SharedLyricsService>,
    font_id: String,
    app: AppHandle,
) -> Result<String, String> {
    let value = service.set_lyrics_font(&font_id)?;
    emit_appearance(&service, &app);
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

fn apply_wallpaper_shell(wallpaper: &WebviewWindow, enabled: bool) -> Result<(), String> {
    if enabled {
        wallpaper
            .set_skip_taskbar(true)
            .map_err(|e| e.to_string())?;
        wallpaper
            .set_ignore_cursor_events(true)
            .map_err(|e| e.to_string())?;
        let _ = wallpaper.set_always_on_top(false);
        let _ = wallpaper.set_focusable(false);
    } else {
        wallpaper
            .set_skip_taskbar(true)
            .map_err(|e| e.to_string())?;
        wallpaper
            .set_ignore_cursor_events(true)
            .map_err(|e| e.to_string())?;
        let _ = wallpaper.set_focusable(false);
    }
    Ok(())
}

/// Detach wallpaper and show the normal overlay (used from focus / handoff paths only).
pub(crate) fn detach_wallpaper_for_ui_focus(app: &AppHandle) {
    if !wallpaper::is_attached() {
        return;
    }
    let Some(main) = app.get_webview_window("main") else {
        startup::step("focus: wallpaper attached but main missing");
        return;
    };
    let flags = app.state::<SharedFlags>();
    let service = app.state::<SharedLyricsService>();
    if let Err(e) = set_wallpaper_mode_inner(&main, flags.inner(), &service, false, true, "focus") {
        startup::step(&format!("focus: wallpaper detach failed: {e}"));
    } else {
        startup::step("focus: wallpaper detached for UI reveal");
    }
}

fn set_wallpaper_mode_inner(
    main: &WebviewWindow,
    flags: &SharedFlags,
    service: &SharedLyricsService,
    enabled: bool,
    persist: bool,
    trigger: &str,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let app = main.app_handle();
        let wallpaper = app
            .get_webview_window("wallpaper")
            .ok_or_else(|| "wallpaper webview missing".to_string())?;
        if enabled {
            let main_hwnd = main.hwnd().map_err(|e| e.to_string())?;
            let saved = save_main_placement(main_hwnd);
            wallpaper::remember_main_placement(saved);
            let _ = main.hide();
            let _ = wallpaper.show();
            apply_wallpaper_shell(&wallpaper, true)?;
            let wp_hwnd = wallpaper.hwnd().map_err(|e| e.to_string())?;
            if let Err(e) = wallpaper::attach_wallpaper(wp_hwnd, main_hwnd, trigger) {
                if persist {
                    let _ = service.set_wallpaper_mode(false);
                }
                let _ = wallpaper.hide();
                let saved = wallpaper::take_main_placement();
                let _ = restore_main_placement(main_hwnd, &saved);
                let _ = main.show();
                let _ = main.unminimize();
                let _ = main.set_focus();
                apply_topmost(main, &flags.lock());
                emit_wallpaper_status(&app);
                let _ = app.emit("wallpaper-attach-failed", &e);
                return Err(e);
            }
            if wallpaper::is_fallback_overlay() {
                let _ = wallpaper.hide();
                let _ = main.show();
                let _ = main.set_ignore_cursor_events(true);
                let _ = main.set_always_on_top(false);
            } else {
                let mut sync_diag = String::new();
                wallpaper_webview::sync_webview_controller(&wallpaper, &mut sync_diag);
                if !sync_diag.is_empty() {
                    startup::log_diagnostic(&format!("WALLPAPER WEBVIEW2 SYNC\n{sync_diag}"));
                }
            }
            if persist {
                service.set_wallpaper_mode(true)?;
            }
        } else {
            let was_fallback = wallpaper::is_fallback_overlay();
            let main_hwnd = main.hwnd().map_err(|e| e.to_string())?;
            let wp_hwnd = wallpaper.hwnd().map_err(|e| e.to_string())?;
            wallpaper::detach_wallpaper(wp_hwnd, main_hwnd, trigger)?;
            if was_fallback {
                let _ = main.set_ignore_cursor_events(false);
            }
            let _ = wallpaper.hide();
            let main_hwnd = main.hwnd().map_err(|e| e.to_string())?;
            let saved = wallpaper::take_main_placement();
            restore_main_placement(main_hwnd, &saved)?;
            let _ = main.show();
            let _ = main.unminimize();
            let _ = main.set_focus();
            apply_wallpaper_shell(&wallpaper, false)?;
            if persist {
                service.set_wallpaper_mode(false)?;
            }
        }
        apply_topmost(main, &flags.lock());
        emit_wallpaper_status(&app);
        if enabled {
            let _ = app.emit("media-state", app.state::<SharedMediaState>().read().clone());
            emit_appearance(&service, &app);
            let _ = app.emit("ui-state", current_ui_state(flags));
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (main, flags, service, enabled, persist);
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
    service: tauri::State<SharedLyricsService>,
    app: AppHandle,
) -> Result<(), String> {
    if let Err(e) =
        set_wallpaper_mode_inner(&window, flags.inner(), &service, enabled, true, "gear")
    {
        let _ = app.emit("ui-state", current_ui_state(flags.inner()));
        return Err(e);
    }
    let _ = app.emit("ui-state", current_ui_state(flags.inner()));
    Ok(())
}

#[tauri::command]
fn toggle_wallpaper_mode(
    window: WebviewWindow,
    flags: tauri::State<SharedFlags>,
    service: tauri::State<SharedLyricsService>,
    app: AppHandle,
) -> Result<bool, String> {
    let next = !wallpaper::is_attached();
    if let Err(e) =
        set_wallpaper_mode_inner(&window, flags.inner(), &service, next, true, "gear")
    {
        let _ = app.emit("ui-state", current_ui_state(flags.inner()));
        return Err(e);
    }
    let _ = app.emit("ui-state", current_ui_state(flags.inner()));
    Ok(next)
}

#[tauri::command]
fn get_wallpaper_status() -> WallpaperStatus {
    build_wallpaper_status()
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WallpaperSurfaceState {
    media: MediaState,
    appearance: AppearanceState,
    sync_offset_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    lyrics: Option<LyricsResponse>,
}

#[tauri::command]
fn get_wallpaper_surface_state(
    media: tauri::State<SharedMediaState>,
    service: tauri::State<SharedLyricsService>,
) -> WallpaperSurfaceState {
    let snap = media.read().clone();
    let appearance = appearance_from_service(&service);
    let sync_offset_ms = if snap.title.is_empty() && snap.artist.is_empty() {
        0
    } else {
        service.get_sync_offset(
            &snap.artist,
            &snap.title,
            &snap.album,
            snap.duration_ms,
        )
    };
    let lyrics = if snap.title.is_empty() && snap.artist.is_empty() {
        None
    } else {
        Some(service.fetch(
            &snap.artist,
            &snap.title,
            &snap.album,
            snap.duration_ms,
        ))
    };
    WallpaperSurfaceState {
        media: snap,
        appearance,
        sync_offset_ms,
        lyrics,
    }
}

#[tauri::command]
fn get_desktop_wallpaper_payload() -> desktop_wallpaper::DesktopWallpaperPayload {
    desktop_wallpaper::desktop_wallpaper_payload()
}

#[cfg(windows)]
#[tauri::command]
fn get_desktop_wallpaper_url() -> Result<String, String> {
    use std::ffi::OsString;
    use std::os::windows::prelude::OsStringExt;
    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER};

    let mut buf = vec![0u16; 4096];
    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            Some(buf.as_mut_ptr() as *mut _),
            Default::default(),
        )
        .map_err(|e| format!("SPI_GETDESKWALLPAPER: {e}"))?;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
  Ok(OsString::from_wide(&buf[..len])
        .to_string_lossy()
        .into_owned())
}

#[cfg(not(windows))]
#[tauri::command]
fn get_desktop_wallpaper_url() -> Result<String, String> {
    Err("desktop wallpaper path is Windows-only".into())
}

#[tauri::command]
fn open_lyricglass_log_folder() -> Result<String, String> {
    let path = wallpaper_log::log_folder();
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let path_str = path.to_string_lossy().into_owned();
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("explorer: {e}"))?;
    }
    #[cfg(not(windows))]
    {
        let _ = path_str.as_str();
    }
    Ok(path_str)
}

#[cfg(not(windows))]
fn run_wallpaper_toggle_on_main(
    _window: &WebviewWindow,
    _app: &AppHandle,
    _enabled: bool,
    _trigger: &'static str,
) {
}

#[cfg(windows)]
fn run_wallpaper_toggle_on_main(
    window: &WebviewWindow,
    app: &AppHandle,
    enabled: bool,
    trigger: &'static str,
) {
    use std::sync::mpsc;
    use std::time::Duration;
    let (tx, rx) = mpsc::sync_channel(1);
    let window_for_main = window.clone();
    let app = app.clone();
    let window_in_closure = window_for_main.clone();
    if window_for_main
        .run_on_main_thread(move || {
            let flags = app.state::<SharedFlags>();
            let service = app.state::<SharedLyricsService>();
            let result = set_wallpaper_mode_inner(
                &window_in_closure,
                flags.inner(),
                &service,
                enabled,
                true,
                trigger,
            );
            let _ = app.emit("ui-state", current_ui_state(flags.inner()));
            emit_wallpaper_status(&app);
            if let Err(ref e) = result {
                let _ = app.emit("wallpaper-attach-failed", e.clone());
            }
            let _ = tx.send(result);
        })
        .is_err()
    {
        startup::step(&format!("wallpaper: run_on_main_thread failed trigger={trigger}"));
        return;
    }
    let _ = rx.recv_timeout(Duration::from_secs(60));
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
                    run_wallpaper_toggle_on_main(
                        &window,
                        &app_handle,
                        !wallpaper::is_attached(),
                        "hotkey",
                    );
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
    let show_window = match MenuItem::with_id(app, "show_window", "Show", true, None::<&str>) {
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
    let menu = match Menu::with_items(app, &[&show_window, &toggle_wallpaper, &toggle_debug, &quit]) {
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
                "show_window" => {
                    let focus_app = app.clone();
                    let _ = app.clone().run_on_main_thread(move || {
                        window_focus::reveal_main_window(&focus_app, "tray-menu-show");
                    });
                }
                "toggle_wallpaper" => {
                    if let Some(window) = app.get_webview_window("main") {
                        run_wallpaper_toggle_on_main(
                            &window,
                            &app_handle,
                            !wallpaper::is_attached(),
                            "tray",
                        );
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
                let focus_app = app.clone();
                let _ = app.clone().run_on_main_thread(move || {
                    window_focus::reveal_main_window(&focus_app, "tray-left-click");
                });
            }
        })
        .build(app)
    {
        startup::step(&format!("tray build failed: {e}"));
    } else {
        startup::step("tray ready");
    }
}

#[cfg(windows)]
fn wallpaper_reattach_from_explorer(app: &AppHandle, trigger: &str) {
    if !wallpaper::is_attached() || wallpaper::is_fallback_overlay() {
        return;
    }
    startup::step(&format!("wallpaper: explorer reattach requested ({trigger})"));
    let Some(main) = app.get_webview_window("main") else {
        return;
    };
    let Some(wallpaper) = app.get_webview_window("wallpaper") else {
        return;
    };
    let Ok(main_hwnd) = main.hwnd() else {
        return;
    };
    let Ok(wp_hwnd) = wallpaper.hwnd() else {
        return;
    };
    use windows::Win32::Foundation::HWND;
    let main_native = HWND(main_hwnd.0 as _);
    let wp_native = HWND(wp_hwnd.0 as _);
    if wallpaper::reattach_desktop(wp_native, main_native, trigger).is_err() {
        startup::step(&format!("wallpaper: reattach failed ({trigger})"));
        return;
    }
    if !wallpaper::is_fallback_overlay() {
        let mut sync_diag = String::new();
        wallpaper_webview::sync_webview_controller(&wallpaper, &mut sync_diag);
    }
    emit_wallpaper_status(app);
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
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
        window_focus::on_second_instance(app, argv, cwd);
    }));
    let run_result = builder
        .plugin({
            startup::step("loading global-shortcut plugin");
            tauri_plugin_global_shortcut::Builder::new().build()
        })
        .manage(media_state.clone())
        .manage(lyrics_service)
        .manage(flags.clone())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    startup::step("main: close (X) — hidden to tray");
                }
            }
        })
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
            set_karaoke_ball_color,
            set_sung_highlight_color,
            reset_karaoke_colors,
            set_karaoke_bouncer,
            set_lyrics_font,
            log_diagnostics,
            log_sync_mark,
            toggle_debug,
            toggle_always_on_top,
            set_wallpaper_mode,
            toggle_wallpaper_mode,
            get_wallpaper_status,
            open_lyricglass_log_folder,
            get_desktop_wallpaper_url,
            get_desktop_wallpaper_payload,
            get_wallpaper_surface_state,
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
                startup::step("startup: overlay mode (do not restore wallpaper)");
                service.ensure_overlay_at_startup();
                startup::step("tauri setup: emitting initial UI state");
                let _ = app.handle().emit("ui-state", current_ui_state(&flags));
                let appearance = app.state::<SharedLyricsService>();
                let _ = app
                    .handle()
                    .emit("appearance-state", appearance_from_service(&appearance));
                startup::step("tauri setup complete — WebView2 loading");
                #[cfg(windows)]
                {
                    use std::sync::Arc;
                    let app_hooks = app.handle().clone();
                    wallpaper_hooks::set_reattach_handler(Arc::new(move |trigger| {
                        wallpaper_reattach_from_explorer(&app_hooks, trigger);
                    }));
                    if let Some(main_w) = app.get_webview_window("main") {
                        if let Ok(hwnd) = main_w.hwnd() {
                            use windows::Win32::Foundation::HWND;
                            wallpaper_hooks::install(HWND(hwnd.0 as _), app.handle().clone());
                        }
                    }
                }
                if std::env::var("LYRICGLASS_SMOKE_DIAG").is_ok() {
                    let app = app.handle().clone();
                    let flags = flags.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        startup::step("smoke: scheduling diagnostics toggle");
                        let app_cb = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            toggle_debug_visible(&app_cb, &flags, "smoke");
                        });
                    });
                }
                #[cfg(windows)]
                if std::env::var("LYRICGLASS_SMOKE_SINGLE_INSTANCE").is_ok() {
                    let app = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(4));
                        let app_hide = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            if let Some(w) = app_hide.get_webview_window("main") {
                                let _ = w.hide();
                                startup::step("smoke: main hidden for single-instance test");
                            }
                        });
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        if let Ok(exe) = std::env::current_exe() {
                            let _ = std::process::Command::new(exe).spawn();
                            startup::step("smoke: second instance launched");
                        }
                    });
                }
                Ok(())
            }
        })
        .build(tauri::generate_context!());

    match run_result {
        Ok(app) => {
            app.run(|_app_handle, event| {
                match &event {
                    RunEvent::Ready => {
                        startup::step("tauri event loop Ready");
                        startup::arm_main_watchdog();
                        startup::start_main_heartbeat_pinger(_app_handle.clone());
                    }
                    _ => {}
                }
            });
            startup::step("tauri exited normally");
        }
        Err(e) => startup::log_fatal_error("tauri build", e),
    }
}