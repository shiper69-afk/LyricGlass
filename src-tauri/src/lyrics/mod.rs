use crate::startup;

mod cache;
mod lrclib;
mod netease;
mod response;
mod shironet;
mod source;

use cache::LyricsCache;
pub use response::{looks_hebrew, LyricsResponse};
use source::should_try_shironet_auto;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct LyricsService {
    cache: Mutex<LyricsCache>,
}

fn has_usable_lyrics(res: &LyricsResponse) -> bool {
    matches!(res.status.as_str(), "synced" | "plain" | "instrumental")
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

    fn try_shironet(artist: &str, track: &str) -> LyricsResponse {
        startup::step("lyrics: trying Shironet fallback (Auto mode)");
        shironet::fetch_from_network(artist, track)
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
            "netease" => netease::fetch_from_network(artist, track, album, duration_ms),
            "lrclib" => {
                let mut lr = lrclib::fetch_from_network(artist, track, album, duration_ms);
                if !has_usable_lyrics(&lr) && lr.status == "not_found" {
                    lr.message = "No lyrics on LRCLIB".into();
                    lr.lyrics_source = "lrclib".into();
                }
                lr
            }
            _ => {
                // Auto: NetEase → LRCLIB → Shironet (Hebrew / not_found only)
                if duration_ms > 0 {
                    let ne = netease::fetch_from_network(artist, track, album, duration_ms);
                    if has_usable_lyrics(&ne) {
                        ne
                    } else {
                        let lr = lrclib::fetch_from_network(artist, track, album, duration_ms);
                        if has_usable_lyrics(&lr) {
                            lr
                        } else if should_try_shironet_auto("auto", artist, track, &lr) {
                            let sh = Self::try_shironet(artist, track);
                            if has_usable_lyrics(&sh) {
                                sh
                            } else if sh.lyrics_source == "shironet" && sh.status == "not_found" {
                                sh
                            } else {
                                lr
                            }
                        } else {
                            lr
                        }
                    }
                } else {
                    let lr = lrclib::fetch_from_network(artist, track, album, duration_ms);
                    if should_try_shironet_auto("auto", artist, track, &lr) {
                        let sh = Self::try_shironet(artist, track);
                        if has_usable_lyrics(&sh) {
                            sh
                        } else {
                            lr
                        }
                    } else {
                        lr
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_allows_shironet_gate_only() {
        let nf = LyricsResponse::not_found();
        assert!(should_try_shironet_auto("auto", "Noah", "Stick", &nf));
        assert!(!should_try_shironet_auto("netease", "Noah", "Stick", &nf));
        assert!(!should_try_shironet_auto("lrclib", "Noah", "Stick", &nf));
    }
}