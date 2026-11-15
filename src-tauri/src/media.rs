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
    /// Raw GSMTC timeline `Position`.
    pub position_reported_ms: u64,
    /// Unix ms from GSMTC `LastUpdatedTime`.
    pub position_reported_at_ms: u64,
    /// Wall anchor for extrapolation when GSMTC refreshes time but not position.
    pub position_extrapolation_wall_ms: u64,
    /// Raw `LastUpdatedTime.UniversalTime` (100 ns ticks since 1601 UTC).
    pub last_updated_raw_ticks: i64,
    /// Host unix ms when this state was published to the UI.
    pub server_time_ms: u64,
    /// Playback position at `server_time_ms` (dual-anchor extrapolation).
    pub computed_position_ms: u64,
    pub playback_rate: f64,
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
            position_extrapolation_wall_ms: 0,
            last_updated_raw_ticks: 0,
            server_time_ms: 0,
            computed_position_ms: 0,
            playback_rate: 1.0,
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
    let playing = state.playback_status.eq_ignore_ascii_case("playing");
    let wall = if state.position_extrapolation_wall_ms > 0 {
        state.position_extrapolation_wall_ms
    } else {
        state.position_reported_at_ms
    };
    clock::media_clock_ms(
        state.position_reported_ms,
        state.position_reported_at_ms,
        wall,
        playing,
        state.playback_rate,
        at_ms,
    )
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
        last_updated_ms: u64,
        last_updated_raw_ticks: i64,
        playback_rate: f64,
        status: String,
        source: String,
    }

    struct ExtrapolationTracker {
        anchor_wall_ms: u64,
    }

    fn snapshots_differ(prev: &Snapshot, snap: &Snapshot) -> bool {
        prev.title != snap.title
            || prev.artist != snap.artist
            || prev.album != snap.album
            || prev.duration_ms != snap.duration_ms
            || prev.position_ms != snap.position_ms
            || prev.status != snap.status
            || prev.last_updated_ms != snap.last_updated_ms
            || (prev.playback_rate - snap.playback_rate).abs() > f64::EPSILON
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
        let playback_info = session.GetPlaybackInfo().ok();
        let status = playback_info
            .as_ref()
            .map(|p| status_label(p.PlaybackStatus().unwrap_or_default()))
            .unwrap_or_else(|| "unknown".into());
        let playback_rate = playback_info
            .as_ref()
            .and_then(|p| p.PlaybackRate().ok())
            .and_then(|reference| reference.Value().ok())
            .filter(|r| r.is_finite() && *r > 0.0)
            .unwrap_or(1.0);

        let (position_ms, duration_ms, last_updated_ms, last_updated_raw_ticks) = session
            .GetTimelineProperties()
            .ok()
            .map(|t| {
                let position_ms = timespan_ms(t.Position().unwrap_or_default());
                let duration_ms = timespan_ms(t.EndTime().unwrap_or_default());
                let last_updated_raw_ticks = t
                    .LastUpdatedTime()
                    .ok()
                    .map(|d| d.UniversalTime)
                    .unwrap_or(0);
                let last_updated_ms = clock::winrt_date_time_to_unix_ms(last_updated_raw_ticks)
                    .unwrap_or(0);
                (position_ms, duration_ms, last_updated_ms, last_updated_raw_ticks)
            })
            .unwrap_or((0, 0, 0, 0));
        let (title, artist, album) = read_properties(session);

        Snapshot {
            title,
            artist,
            album,
            duration_ms,
            position_ms,
            last_updated_ms,
            last_updated_raw_ticks,
            playback_rate,
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
        let mut extrapolation: Option<ExtrapolationTracker> = None;
        let mut last_publish_ms: u64 = 0;

        loop {
                let message = if let Some(manager) = request_manager() {
                    if let Some(session) = find_spotify_session(&manager) {
                        let snap = capture_snapshot(&session);
                        let changed = last.as_ref().map_or(true, |p| snapshots_differ(p, &snap));
                        let host_now = now_unix_ms();
                        let playing_heartbeat = snap.status == "playing"
                            && host_now.saturating_sub(last_publish_ms) >= 1000;

                        if changed || playing_heartbeat {
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
                                } else if !changed {
                                    event_kind = "heartbeat";
                                }
                            }

                            // Reset wall only on track/playback/seek — not on every GSMTC tick
                            // (Store Spotify advances Position slowly while audio runs ahead).
                            let needs_wall_reset = last.is_none()
                                || event_kind == "track"
                                || event_kind == "playback"
                                || seek_jump;

                            if needs_wall_reset {
                                extrapolation = Some(ExtrapolationTracker {
                                    anchor_wall_ms: host_now,
                                });
                            } else if extrapolation.is_none() {
                                extrapolation = Some(ExtrapolationTracker {
                                    anchor_wall_ms: host_now,
                                });
                            }

                            let gsmtc_anchor = if snap.last_updated_ms > 0 {
                                snap.last_updated_ms
                            } else {
                                host_now
                            };
                            let wall_anchor = extrapolation
                                .as_ref()
                                .map(|e| e.anchor_wall_ms)
                                .unwrap_or(host_now);

                            let mut published = shared.read().clone();
                            published.spotify_running = true;
                            published.source_app_id = snap.source.clone();
                            published.title = snap.title.clone();
                            published.artist = snap.artist.clone();
                            published.album = snap.album.clone();
                            published.duration_ms = snap.duration_ms;
                            published.position_reported_ms = snap.position_ms;
                            published.position_reported_at_ms = gsmtc_anchor;
                            published.position_extrapolation_wall_ms = wall_anchor;
                            published.last_updated_raw_ticks = snap.last_updated_raw_ticks;
                            published.server_time_ms = host_now;
                            published.playback_rate = snap.playback_rate;
                            published.playback_status = snap.status.clone();
                            published.last_event_kind = event_kind.to_string();
                            published.message = "Spotify session active".into();
                            published.computed_position_ms =
                                smoothed_now(&published, host_now);
                            {
                                let mut state = shared.write();
                                if timeline_tick {
                                    state.timeline_updates =
                                        state.timeline_updates.saturating_add(1);
                                }
                                published.timeline_updates = state.timeline_updates;
                                *state = published.clone();
                            }

                            last = Some(snap);
                            last_publish_ms = host_now;
                            let _ = app.emit("media-state", published);
                            if seek_jump {
                                let _ = app.emit("seek-tick", ());
                            }
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