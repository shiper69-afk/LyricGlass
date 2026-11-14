use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

use crate::clock;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub spotify_running: bool,
    pub source_app_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub position_reported_ms: u64,
    pub position_reported_at_ms: u64,
    pub playback_status: String,
    pub last_event_kind: String,
    pub timeline_updates: u64,
    pub message: String,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            spotify_running: false,
            source_app_id: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            duration_ms: 0,
            position_reported_ms: 0,
            position_reported_at_ms: 0,
            playback_status: "stopped".into(),
            last_event_kind: "init".into(),
            timeline_updates: 0,
            message: "Waiting for Windows media session…".into(),
        }
    }
}

pub type SharedMediaState = Arc<RwLock<MediaState>>;

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn smoothed_now(state: &MediaState, at_ms: u64) -> u64 {
    let elapsed = at_ms.saturating_sub(state.position_reported_at_ms);
    let playing = state.playback_status.eq_ignore_ascii_case("playing");
    clock::smoothed_position_ms(state.position_reported_ms, playing, elapsed)
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use windows::Foundation::TypedEventHandler;
    use windows_future::Future;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    static TIMELINE_COUNT: AtomicU64 = AtomicU64::new(0);

    fn status_label(status: GlobalSystemMediaTransportControlsSessionPlaybackStatus) -> String {
        match status {
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => {
                "playing".to_string()
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "paused".to_string(),
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped => {
                "stopped".to_string()
            }
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing => {
                "changing".to_string()
            }
            _ => format!("other({status:?})"),
        }
    }

    fn patch_state<F>(shared: &SharedMediaState, event_kind: &str, f: F)
    where
        F: FnOnce(&mut MediaState),
    {
        let mut state = shared.write();
        state.last_event_kind = event_kind.to_string();
        if event_kind == "timeline" {
            state.timeline_updates = TIMELINE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        }
        f(&mut state);
    }

  async fn read_session(session: &GlobalSystemMediaTransportControlsSession, shared: SharedMediaState) {
        let source = session
            .SourceAppUserModelId()
            .unwrap_or_default()
            .to_string();

        let playback = session.GetPlaybackInfo().ok();
        let playback_status = playback
            .as_ref()
            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
            .unwrap_or_else(|| "unknown".into());

        let timeline = session.GetTimelineProperties().ok();
        let position_ms = timeline
            .as_ref()
            .map(|t| t.Position().unwrap_or_default().Duration / 10_000)
            .unwrap_or(0);
        let end_ms = timeline
            .as_ref()
            .map(|t| t.EndTime().unwrap_or_default().Duration / 10_000)
            .unwrap_or(0);

        let props = session.TryGetMediaPropertiesAsync().ok();
        let (title, artist, album) = if let Some(op) = props {
            if let Ok(props) = op.await {
                (
                    props.Title().unwrap_or_default().to_string(),
                    props.Artist().unwrap_or_default().to_string(),
                    props.AlbumTitle().unwrap_or_default().to_string(),
                )
            } else {
                (String::new(), String::new(), String::new())
            }
        } else {
            (String::new(), String::new(), String::new())
        };

        patch_state(&shared, "snapshot", |state| {
            state.spotify_running = true;
            state.source_app_id = source;
            state.title = title;
            state.artist = artist;
            state.album = album;
            state.duration_ms = end_ms;
            state.position_reported_ms = position_ms;
            state.position_reported_at_ms = now_unix_ms();
            state.playback_status = playback_status;
            state.message = "Spotify session active".into();
        });
    }

    fn find_spotify_session(
        manager: &GlobalSystemMediaTransportControlsSessionManager,
    ) -> Option<GlobalSystemMediaTransportControlsSession> {
        let sessions = manager.GetSessions().ok()?;
        for session in sessions {
            if let Ok(id) = session.SourceAppUserModelId() {
                let id = id.to_string();
                if id.to_lowercase().contains("spotify") {
                    return Some(session);
                }
            }
        }
        None
    }

    fn wire_session_events(
        session: GlobalSystemMediaTransportControlsSession,
        shared: SharedMediaState,
        app: AppHandle,
    ) {
        let shared_timeline = shared.clone();
        let app_timeline = app.clone();
        let _ = session.TimelineUpdated(&TypedEventHandler::new(
            move |sender, _args| {
                if let Some(sender) = sender {
                    let shared = shared_timeline.clone();
                    let app = app_timeline.clone();
                    tauri::async_runtime::spawn(async move {
                        let playback = sender.GetPlaybackInfo().ok();
                        let status = playback
                            .as_ref()
                            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
                            .unwrap_or_else(|| "unknown".into());
                        let timeline = sender.GetTimelineProperties().ok();
                        let position_ms = timeline
                            .as_ref()
                            .map(|t| t.Position().unwrap_or_default().Duration / 10_000)
                            .unwrap_or(0);
                        let end_ms = timeline
                            .as_ref()
                            .map(|t| t.EndTime().unwrap_or_default().Duration / 10_000)
                            .unwrap_or(0);

                        patch_state(&shared, "timeline", |state| {
                            state.position_reported_ms = position_ms;
                            state.position_reported_at_ms = now_unix_ms();
                            state.duration_ms = end_ms;
                            state.playback_status = status;
                        });
                        let payload = shared.read().clone();
                        let _ = app.emit("media-state", payload);
                    });
                }
                Ok(())
            },
        ));

        let shared_playback = shared.clone();
        let app_playback = app.clone();
        let _ = session.PlaybackInfoChanged(&TypedEventHandler::new(
            move |sender, _args| {
                if let Some(sender) = sender {
                    let shared = shared_playback.clone();
                    let app = app_playback.clone();
                    tauri::async_runtime::spawn(async move {
                        let playback = sender.GetPlaybackInfo().ok();
                        let status = playback
                            .as_ref()
                            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
                            .unwrap_or_else(|| "unknown".into());
                        patch_state(&shared, "playback", |state| {
                            state.playback_status = status;
                        });
                        let _ = app.emit("media-state", shared.read().clone());
                    });
                }
                Ok(())
            },
        ));

        let shared_props = shared.clone();
        let app_props = app.clone();
        let _ = session.MediaPropertiesChanged(&TypedEventHandler::new(
            move |sender, _args| {
                if let Some(sender) = sender {
                    let shared = shared_props.clone();
                    let app = app_props.clone();
                    tauri::async_runtime::spawn(async move {
                        let props = sender.TryGetMediaPropertiesAsync().ok();
                        let (title, artist, album) = if let Some(op) = props {
                            if let Ok(props) = op.await {
                                (
                                    props.Title().unwrap_or_default().to_string(),
                                    props.Artist().unwrap_or_default().to_string(),
                                    props.AlbumTitle().unwrap_or_default().to_string(),
                                )
                            } else {
                                (String::new(), String::new(), String::new())
                            }
                        } else {
                            (String::new(), String::new(), String::new())
                        };
                        patch_state(&shared, "track", |state| {
                            state.title = title;
                            state.artist = artist;
                            state.album = album;
                            state.position_reported_ms = 0;
                            state.position_reported_at_ms = now_unix_ms();
                        });
                        let _ = app.emit("media-state", shared.read().clone());
                    });
                }
                Ok(())
            },
        ));
    }

    pub fn start_media_watcher(app: AppHandle, shared: SharedMediaState) {
        tauri::async_runtime::spawn(async move {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }

            let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
                Ok(op) => match op.await {
                    Ok(m) => m,
                    Err(e) => {
                        shared.write().message = format!("GSMTC unavailable: {e}");
                        return;
                    }
                },
                Err(e) => {
                    shared.write().message = format!("GSMTC request failed: {e}");
                    return;
                }
            };

            let shared_mgr = shared.clone();
            let app_mgr = app.clone();
            let _ = manager.SessionsChanged(&TypedEventHandler::new(
                move |_manager, _args| {
                    let shared = shared_mgr.clone();
                    let app = app_mgr.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Ok(m) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                        {
                            if let Ok(manager) = m.await {
                                if let Some(session) = find_spotify_session(&manager) {
                                    wire_session_events(session.clone(), shared.clone(), app.clone());
                                    read_session(&session, shared.clone()).await;
                                    let _ = app.emit("media-state", shared.read().clone());
                                } else {
                                    let mut state = shared.write();
                                    state.spotify_running = false;
                                    state.message =
                                        "Spotify not in media sessions (launch Spotify and play)".into();
                                    let _ = app.emit("media-state", state.clone());
                                }
                            }
                        }
                    });
                    Ok(())
                },
            ));

            if let Some(session) = find_spotify_session(&manager) {
                wire_session_events(session.clone(), shared.clone(), app.clone());
                read_session(&session, shared).await;
                let _ = app.emit("media-state", shared.read().clone());
            } else {
                shared.write().message =
                    "Spotify not found — start Spotify desktop and play a track".into();
                let _ = app.emit("media-state", shared.read().clone());
            }
        });
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn start_media_watcher(app: AppHandle, shared: SharedMediaState) {
        shared.write().message = "Media controls SPIKE runs on Windows only".into();
        let _ = app.emit("media-state", shared.read().clone());
    }
}

pub use imp::start_media_watcher;