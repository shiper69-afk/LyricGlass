use crate::startup;

mod cache;
mod lrclib;
mod netease;
mod response;
mod shironet;

use cache::LyricsCache;
pub use response::{looks_hebrew, LyricsResponse};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct LyricsService {
    cache: Mutex<LyricsCache>,
}

fn lrclib_has_lyrics(res: &LyricsResponse) -> bool {
    matches!(res.status.as_str(), "synced" | "plain" | "instrumental")
}

fn netease_has_lyrics(res: &LyricsResponse) -> bool {
    matches!(res.status.as_str(), "synced")
}

fn should_try_shironet(artist: &str, track: &str, primary: &LyricsResponse) -> bool {
    if lrclib_has_lyrics(primary) || netease_has_lyrics(primary) {
        return false;
    }
    looks_hebrew(artist)
        || looks_hebrew(track)
        || matches!(primary.status.as_str(), "not_found")
}

fn apply_shironet_if_needed(
    artist: &str,
    track: &str,
    primary: LyricsResponse,
) -> LyricsResponse {
    if !should_try_shironet(artist, track, &primary) {
        return primary;
    }
    startup::step("lyrics: trying Shironet fallback");
    let shironet = shironet::fetch_from_network(artist, track);
    if shironet.status == "plain" && shironet.plain_lyrics.is_some() {
        return shironet;
    }
    if primary.status == "not_found" && shironet.lyrics_source == "shironet" {
        return shironet;
    }
    primary
}

impl LyricsService {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            cache: Mutex::new(LyricsCache::open()?),
        })
    }

    pub fn get_lyrics_source(&self) -> String {
        self.cache.lock().get_lyrics_source()
    }

    pub fn set_lyrics_source(&self, mode: &str) -> Result<String, String> {
        self.cache.lock().set_lyrics_source(mode)
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

    pub fn get_wallpaper_mode(&self) -> bool {
        self.cache.lock().get_wallpaper_mode()
    }

    pub fn set_wallpaper_mode(&self, enabled: bool) -> Result<bool, String> {
        self.cache.lock().set_wallpaper_mode(enabled)
    }

    pub fn ensure_overlay_at_startup(&self) {
        self.cache.lock().ensure_overlay_at_startup();
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

    pub fn get_lyrics_font(&self) -> String {
        self.cache.lock().get_lyrics_font()
    }

    pub fn set_lyrics_font(&self, font_id: &str) -> Result<String, String> {
        self.cache.lock().set_lyrics_font(font_id)
    }

    pub fn get_visualizer_kind(&self) -> String {
        self.cache.lock().get_visualizer_kind()
    }

    pub fn set_visualizer_kind(&self, kind: &str) -> Result<String, String> {
        self.cache.lock().set_visualizer_kind(kind)
    }

    pub fn get_visualizer_intensity(&self) -> f64 {
        self.cache.lock().get_visualizer_intensity()
    }

    pub fn set_visualizer_intensity(&self, value: f64) -> Result<f64, String> {
        self.cache.lock().set_visualizer_intensity(value)
    }

    pub fn get_visualizer_opacity(&self) -> f64 {
        self.cache.lock().get_visualizer_opacity()
    }

    pub fn set_visualizer_opacity(&self, value: f64) -> Result<f64, String> {
        self.cache.lock().set_visualizer_opacity(value)
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
        let mode = self.get_lyrics_source();
        if let Some(hit) = self
            .cache
            .lock()
            .get(artist, track, album, duration_ms, &mode)
        {
            startup::step("lyrics: cache hit");
            return hit;
        }

        let result = match mode.as_str() {
            "netease" => {
                let ne = netease::fetch_from_network(artist, track, album, duration_ms);
                apply_shironet_if_needed(artist, track, ne)
            }
            "lrclib" => {
                let lr = lrclib::fetch_from_network(artist, track, album, duration_ms);
                apply_shironet_if_needed(artist, track, lr)
            }
            _ => {
                // Auto: NetEase word-level when strict match, else LRCLIB, else Shironet
                if duration_ms > 0 {
                    let ne = netease::fetch_from_network(artist, track, album, duration_ms);
                    if netease_has_lyrics(&ne) {
                        ne
                    } else {
                        let lr = lrclib::fetch_from_network(artist, track, album, duration_ms);
                        apply_shironet_if_needed(artist, track, lr)
                    }
                } else {
                    let lr = lrclib::fetch_from_network(artist, track, album, duration_ms);
                    apply_shironet_if_needed(artist, track, lr)
                }
            }
        };

        if let Err(e) = self.cache.lock().put(
            artist,
            track,
            album,
            duration_ms,
            &mode,
            &result,
        ) {
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