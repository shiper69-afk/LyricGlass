use crate::startup;

mod cache;
mod lrclib;

use cache::LyricsCache;
pub use lrclib::LyricsResponse;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct LyricsService {
    cache: Mutex<LyricsCache>,
}

impl LyricsService {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            cache: Mutex::new(LyricsCache::open()?),
        })
    }

    pub fn get_sync_offset(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
    ) -> i64 {
        self.cache.lock().get_sync_offset(artist, track, album, duration_ms)
    }

    pub fn set_sync_offset(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        offset_ms: i64,
    ) -> Result<i64, String> {
        let clamped = offset_ms.clamp(-10_000, 10_000);
        self.cache
            .lock()
            .set_sync_offset(artist, track, album, duration_ms, clamped)?;
        Ok(clamped)
    }

    pub fn get_overlay_opacity(&self) -> f64 {
        self.cache.lock().get_overlay_opacity()
    }

    pub fn set_overlay_opacity(&self, opacity: f64) -> Result<f64, String> {
        self.cache.lock().set_overlay_opacity(opacity)
    }

    pub fn get_always_on_top(&self) -> bool {
        self.cache.lock().get_always_on_top()
    }

    pub fn set_always_on_top(&self, enabled: bool) -> Result<bool, String> {
        self.cache.lock().set_always_on_top(enabled)
    }

    pub fn get_karaoke_ball(&self) -> bool {
        self.cache.lock().get_karaoke_ball()
    }

    pub fn set_karaoke_ball(&self, enabled: bool) -> Result<bool, String> {
        self.cache.lock().set_karaoke_ball(enabled)
    }

    pub fn get_karaoke_ball_color(&self) -> String {
        self.cache.lock().get_karaoke_ball_color()
    }

    pub fn get_sung_highlight_color(&self) -> String {
        self.cache.lock().get_sung_highlight_color()
    }

    pub fn set_karaoke_ball_color(&self, color: &str) -> Result<String, String> {
        self.cache.lock().set_karaoke_ball_color(color)
    }

    pub fn set_sung_highlight_color(&self, color: &str) -> Result<String, String> {
        self.cache.lock().set_sung_highlight_color(color)
    }

    pub fn reset_karaoke_colors(&self) -> Result<(), String> {
        self.cache.lock().reset_karaoke_colors()
    }

    pub fn get_karaoke_bouncer(&self) -> String {
        self.cache.lock().get_karaoke_bouncer()
    }

    pub fn set_karaoke_bouncer(&self, bouncer_id: &str) -> Result<String, String> {
        self.cache.lock().set_karaoke_bouncer(bouncer_id)
    }

    pub fn add_sync_offset(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        delta_ms: i64,
    ) -> Result<i64, String> {
        self.cache
            .lock()
            .add_sync_offset(artist, track, album, duration_ms, delta_ms)
    }

    pub fn fetch(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
    ) -> LyricsResponse {
        if let Some(hit) = self.cache.lock().get(artist, track, album, duration_ms) {
            startup::step("lyrics: cache hit");
            return hit;
        }

        let result = lrclib::fetch_from_network(artist, track, album, duration_ms);
        if let Err(e) = self.cache.lock().put(artist, track, album, duration_ms, &result) {
            startup::step(&format!("lyrics: cache write failed: {e}"));
        }
        result
    }
}

pub type SharedLyricsService = Arc<LyricsService>;

pub fn create_service() -> SharedLyricsService {
    match LyricsService::new() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            startup::step(&format!("lyrics cache disabled: {e}"));
            Arc::new(LyricsService {
                cache: Mutex::new(LyricsCache::disabled()),
            })
        }
    }
}