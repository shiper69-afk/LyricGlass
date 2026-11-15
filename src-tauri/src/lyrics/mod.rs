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