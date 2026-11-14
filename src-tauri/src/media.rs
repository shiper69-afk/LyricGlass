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
    use std::time::Duration;
    use windows::Foundation::TypedEventHandler;
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

    fn timespan_ms(value: windows::Foundation::TimeSpan) -> u64 {
        (value.Duration / 10_000) as u64
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

    macro_rules! wait_async {
        ($op:expr) => {{
            let operation = $op;
            let mut value = None;
            for _ in 0..400 {
                if let Ok(result) = operation.GetResults() {
                    value = Some(result);
                    break;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            value.or_else(|| operation.GetResults().ok())
        }};
    }

    fn read_properties(session: &GlobalSystemMediaTransportControlsSession) -> (String, String, String) {
        session
            .TryGetMediaPropertiesAsync()
            .ok()
            .and_then(|op| wait_async!(op))
            .map(|props| {
                (
                    props.Title().unwrap_or_default().to_string(),
                    props.Artist().unwrap_or_default().to_string(),
                    props.AlbumTitle().unwrap_or_default().to_string(),
                )
            })
            .unwrap_or_default()
    }

    fn read_snapshot(session: &GlobalSystemMediaTransportControlsSession, shared: &SharedMediaState) {
        let source = session
            .SourceAppUserModelId()
            .unwrap_or_default()
            .to_string();

        let playback_status = session
            .GetPlaybackInfo()
            .ok()
            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
            .unwrap_or_else(|| "unknown".into());

        let (position_ms, end_ms) = session
            .GetTimelineProperties()
            .ok()
            .map(|t| {
                (
                    timespan_ms(t.Position().unwrap_or_default()),
                    timespan_ms(t.EndTime().unwrap_or_default()),
                )
            })
            .unwrap_or((0, 0));

        let (title, artist, album) = read_properties(session);

        patch_state(shared, "snapshot", |state| {
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

    fn apply_timeline(session: &GlobalSystemMediaTransportControlsSession, shared: &SharedMediaState) {
        let playback_status = session
            .GetPlaybackInfo()
            .ok()
            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
            .unwrap_or_else(|| "unknown".into());

        let (position_ms, end_ms) = session
            .GetTimelineProperties()
            .ok()
            .map(|t| {
                (
                    timespan_ms(t.Position().unwrap_or_default()),
                    timespan_ms(t.EndTime().unwrap_or_default()),
                )
            })
            .unwrap_or((0, 0));

        patch_state(shared, "timeline", |state| {
            state.spotify_running = true;
            state.position_reported_ms = position_ms;
            state.position_reported_at_ms = now_unix_ms();
            state.duration_ms = end_ms;
            state.playback_status = playback_status;
        });
    }

    fn find_spotify_session(
        manager: &GlobalSystemMediaTransportControlsSessionManager,
    ) -> Option<GlobalSystemMediaTransportControlsSession> {
        let sessions = manager.GetSessions().ok()?;
        for session in sessions {
            if let Ok(id) = session.SourceAppUserModelId() {
                if id.to_string().to_lowercase().contains("spotify") {
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
        let _ = session.TimelinePropertiesChanged(&TypedEventHandler::new(
            move |sender, _args| {
                apply_timeline(&sender, &shared_timeline);
                let _ = app_timeline.emit("media-state", shared_timeline.read().clone());
                Ok(())
            },
        ));

        let shared_playback = shared.clone();
        let app_playback = app.clone();
        let _ = session.PlaybackInfoChanged(&TypedEventHandler::new(
            move |sender, _args| {
                let status = sender
                    .GetPlaybackInfo()
                    .ok()
                    .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
                    .unwrap_or_else(|| "unknown".into());
                patch_state(&shared_playback, "playback", |state| {
                    state.spotify_running = true;
                    state.playback_status = status;
                });
                let _ = app_playback.emit("media-state", shared_playback.read().clone());
                Ok(())
            },
        ));

        let shared_props = shared.clone();
        let app_props = app.clone();
        let _ = session.MediaPropertiesChanged(&TypedEventHandler::new(
            move |sender, _args| {
                let (title, artist, album) = read_properties(&sender);
                patch_state(&shared_props, "track", |state| {
                    state.spotify_running = true;
                    state.title = title;
                    state.artist = artist;
                    state.album = album;
                    state.position_reported_ms = 0;
                    state.position_reported_at_ms = now_unix_ms();
                });
                let _ = app_props.emit("media-state", shared_props.read().clone());
                Ok(())
            },
        ));
    }

    fn request_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
        let op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?;
        wait_async!(op)
    }

    pub fn start_media_watcher(app: AppHandle, shared: SharedMediaState) {
        std::thread::spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }

            let Some(manager) = request_manager() else {
                shared.write().message = "GSMTC request failed".into();
                let _ = app.emit("media-state", shared.read().clone());
                return;
            };

            let shared_mgr = shared.clone();
            let app_mgr = app.clone();
            let _ = manager.SessionsChanged(&TypedEventHandler::new(
                move |_manager, _args| {
                    let shared = shared_mgr.clone();
                    let app = app_mgr.clone();
                    if let Some(manager) = request_manager() {
                        if let Some(session) = find_spotify_session(&manager) {
                            wire_session_events(session.clone(), shared.clone(), app.clone());
                            read_snapshot(&session, &shared);
                            let _ = app.emit("media-state", shared.read().clone());
                        } else {
                            let mut state = shared.write();
                            state.spotify_running = false;
                            state.message =
                                "Spotify not in media sessions (launch Spotify and play)".into();
                            let _ = app.emit("media-state", state.clone());
                        }
                    }
                    Ok(())
                },
            ));

            if let Some(session) = find_spotify_session(&manager) {
                wire_session_events(session.clone(), shared.clone(), app.clone());
                read_snapshot(&session, &shared);
                let _ = app.emit("media-state", shared.read().clone());
            } else {
                shared.write().message =
                    "Spotify not found — start Spotify desktop and play a track".into();
                let _ = app.emit("media-state", shared.read().clone());
            }

            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        });
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn start_media_watcher(app: AppHandle, shared: SharedMediaState) {
        shared.write().message = "Media controls spike runs on Windows only".into();
        let _ = app.emit("media-state", shared.read().clone());
    }
}

pub use imp::start_media_watcher;