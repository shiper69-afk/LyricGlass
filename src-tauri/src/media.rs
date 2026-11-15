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
    use std::time::Duration;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    struct Snapshot {
        title: String,
        artist: String,
        album: String,
        duration_ms: u64,
        position_ms: u64,
        status: String,
        source: String,
    }

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
            _ => "unknown".to_string(),
        }
    }

    fn timespan_ms(value: windows::Foundation::TimeSpan) -> u64 {
        (value.Duration / 10_000) as u64
    }

    macro_rules! wait_async {
        ($op:expr) => {{
            let operation = $op;
            let mut value = None;
            for _ in 0..200 {
                if let Ok(result) = operation.GetResults() {
                    value = Some(result);
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
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

    fn capture_snapshot(session: &GlobalSystemMediaTransportControlsSession) -> Snapshot {
        let source = session
            .SourceAppUserModelId()
            .unwrap_or_default()
            .to_string();
        let status = session
            .GetPlaybackInfo()
            .ok()
            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
            .unwrap_or_else(|| "unknown".into());
        let (position_ms, duration_ms) = session
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

        Snapshot {
            title,
            artist,
            album,
            duration_ms,
            position_ms,
            status,
            source,
        }
    }

    fn find_spotify_session(
        manager: &GlobalSystemMediaTransportControlsSessionManager,
    ) -> Option<GlobalSystemMediaTransportControlsSession> {
        let sessions = manager.GetSessions().ok()?;
        for session in sessions {
            if let Ok(id) = session.SourceAppUserModelId() {
                let id_lower = id.to_string().to_lowercase();
                if id_lower.contains("spotify") || id_lower.contains("spotifyab") {
                    return Some(session);
                }
            }
        }
        None
    }

    fn request_manager() -> Option<GlobalSystemMediaTransportControlsSessionManager> {
        let op = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?;
        wait_async!(op)
    }

    fn watcher_loop(app: AppHandle, shared: SharedMediaState) {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            crate::startup::step(&format!("media watcher: CoInitializeEx -> {hr:?}"));
        }

        let mut last: Option<Snapshot> = None;

        loop {
                let message = if let Some(manager) = request_manager() {
                    if let Some(session) = find_spotify_session(&manager) {
                        let snap = capture_snapshot(&session);
                        let mut event_kind = "poll";
                        let mut timeline_tick = false;

                        let mut seek_jump = false;
                        if let Some(prev) = &last {
                            if prev.title != snap.title
                                || prev.artist != snap.artist
                                || prev.album != snap.album
                            {
                                event_kind = "track";
                            } else if prev.status != snap.status {
                                event_kind = "playback";
                            } else if prev.position_ms != snap.position_ms {
                                event_kind = "timeline";
                                timeline_tick = true;
                                seek_jump = prev.position_ms.abs_diff(snap.position_ms) > 2000;
                            }
                        }

                        {
                            let mut state = shared.write();
                            if timeline_tick {
                                state.timeline_updates += 1;
                            }
                            state.spotify_running = true;
                            state.source_app_id = snap.source.clone();
                            state.title = snap.title.clone();
                            state.artist = snap.artist.clone();
                            state.album = snap.album.clone();
                            state.duration_ms = snap.duration_ms;
                            state.position_reported_ms = snap.position_ms;
                            state.position_reported_at_ms = now_unix_ms();
                            state.playback_status = snap.status.clone();
                            state.last_event_kind = event_kind.to_string();
                            state.message = "Spotify session active".into();
                        }

                        last = Some(snap);
                        let _ = app.emit("media-state", shared.read().clone());
                        if seek_jump {
                            let _ = app.emit("seek-tick", ());
                        }
                        None
                    } else {
                        last = None;
                        Some("Spotify not in media sessions (launch Spotify and play)".to_string())
                    }
                } else {
                    last = None;
                    Some("GSMTC unavailable".to_string())
                };

                if let Some(msg) = message {
                    {
                        let mut state = shared.write();
                        state.spotify_running = false;
                        state.message = msg;
                        state.last_event_kind = "sessions".into();
                    }
                    let _ = app.emit("media-state", shared.read().clone());
                }

                std::thread::sleep(Duration::from_millis(250));
            }
    }

    pub fn start_media_watcher(app: AppHandle, shared: SharedMediaState) {
        std::thread::spawn(move || {
            crate::startup::step("media watcher thread spawned");
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                watcher_loop(app, shared);
            })) {
                Ok(_) => crate::startup::step("media watcher thread ended"),
                Err(_) => crate::startup::step("media watcher thread panicked (caught)"),
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