use super::response::LyricsResponse;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const FOUND_TTL_SECS: i64 = 7 * 24 * 3600;
const NOT_FOUND_TTL_SECS: i64 = 6 * 3600;
/// Built-in compensation for Microsoft Store Spotify GSMTC lag (v0.6.1+).
pub const SYNC_BUILTIN_LAG_MS: i64 = 600;

pub struct LyricsCache {
    conn: Option<Connection>,
}

impl LyricsCache {
    pub fn open() -> Result<Self, String> {
        let path = cache_db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS lyrics_cache (
                cache_key TEXT PRIMARY KEY,
                response_json TEXT NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sync_offset (
                cache_key TEXT PRIMARY KEY,
                offset_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;
        Self::migrate_sync_offsets_v061(&conn)?;
        Ok(Self { conn: Some(conn) })
    }

    fn migrate_sync_offsets_v061(conn: &Connection) -> Result<(), String> {
        let migrated: bool = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'sync_offset_v061_migrated'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .map(|v| v == "1")
            .unwrap_or(false);
        if migrated {
            return Ok(());
        }
        conn.execute(
            "UPDATE sync_offset SET offset_ms = MAX(-10000, MIN(10000, offset_ms - ?1))",
            params![SYNC_BUILTIN_LAG_MS],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('sync_offset_v061_migrated', '1')",
            [],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn disabled() -> Self {
        Self { conn: None }
    }

    fn track_key(artist: &str, track: &str, album: &str, duration_ms: u64) -> String {
        let norm = |s: &str| s.trim().to_lowercase();
        let duration_sec = duration_ms / 1000;
        format!(
            "{}|{}|{}|{duration_sec}",
            norm(artist),
            norm(track),
            norm(album)
        )
    }

    fn lyrics_cache_key(
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        lyrics_source: &str,
    ) -> String {
        format!(
            "{}|{}",
            Self::track_key(artist, track, album, duration_ms),
            lyrics_source.trim().to_lowercase()
        )
    }

    fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    pub fn get(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        lyrics_source: &str,
    ) -> Option<LyricsResponse> {
        let conn = self.conn.as_ref()?;
        let key = Self::lyrics_cache_key(artist, track, album, duration_ms, lyrics_source);
        let now = Self::now();
        let mut stmt = conn
            .prepare("SELECT response_json FROM lyrics_cache WHERE cache_key = ?1 AND expires_at > ?2")
            .ok()?;
        let json: String = stmt.query_row(params![key, now], |row| row.get(0)).ok()?;
        let mut response: LyricsResponse = serde_json::from_str(&json).ok()?;
        response.from_cache = true;
        Some(response)
    }

    pub fn get_sync_offset(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
    ) -> i64 {
        let conn = match &self.conn {
            Some(c) => c,
            None => return 0,
        };
        let key = Self::track_key(artist, track, album, duration_ms);
        conn.query_row(
            "SELECT offset_ms FROM sync_offset WHERE cache_key = ?1",
            params![key],
            |row| row.get(0),
        )
        .unwrap_or(0)
    }

    pub fn set_sync_offset(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        offset_ms: i64,
    ) -> Result<(), String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(()),
        };
        let key = Self::track_key(artist, track, album, duration_ms);
        conn.execute(
            "INSERT OR REPLACE INTO sync_offset (cache_key, offset_ms) VALUES (?1, ?2)",
            params![key, offset_ms],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn read_setting_f64(conn: &Connection, key: &str) -> Option<f64> {
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
    }

    /// Background fill opacity 0.0 (transparent) .. 1.0 (solid). Text is not dimmed.
    pub fn get_overlay_opacity(&self) -> f64 {
        const DEFAULT_BG: f64 = 0.38;
        let conn = match &self.conn {
            Some(c) => c,
            None => return DEFAULT_BG,
        };
        if let Some(v) = Self::read_setting_f64(conn, "background_opacity") {
            return v.clamp(0.0, 1.0);
        }
        if let Some(legacy) = Self::read_setting_f64(conn, "overlay_opacity") {
            // Pre–v0.2.4 stored UI "panel" strength (0.30–1.0); rendered as panel * 0.62.
            let mapped = if (0.30..=1.0).contains(&legacy) {
                legacy * 0.62
            } else {
                legacy
            };
            return mapped.clamp(0.0, 1.0);
        }
        DEFAULT_BG
    }

    pub fn get_always_on_top(&self) -> bool {
        let conn = match &self.conn {
            Some(c) => c,
            None => return true,
        };
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'always_on_top'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(true)
    }

    pub fn set_always_on_top(&self, enabled: bool) -> Result<bool, String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(enabled),
        };
        let value = if enabled { "1" } else { "0" };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('always_on_top', ?1)",
            params![value],
        )
        .map_err(|e| e.to_string())?;
        Ok(enabled)
    }

    pub fn get_wallpaper_mode(&self) -> bool {
        let conn = match &self.conn {
            Some(c) => c,
            None => return false,
        };
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'wallpaper_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
    }

    /// Launch always in overlay mode even when `wallpaper_mode` was left on in settings.
    pub fn ensure_overlay_at_startup(&self) {
        if self.get_wallpaper_mode() {
            let _ = self.set_wallpaper_mode(false);
        }
    }

    pub fn set_wallpaper_mode(&self, enabled: bool) -> Result<bool, String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(enabled),
        };
        let value = if enabled { "1" } else { "0" };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('wallpaper_mode', ?1)",
            params![value],
        )
        .map_err(|e| e.to_string())?;
        Ok(enabled)
    }

    pub fn get_karaoke_ball(&self) -> bool {
        let conn = match &self.conn {
            Some(c) => c,
            None => return true,
        };
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'karaoke_ball'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .map(|v| !matches!(v.as_str(), "0" | "false" | "no"))
        .unwrap_or(true)
    }

    pub fn set_karaoke_ball(&self, enabled: bool) -> Result<bool, String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(enabled),
        };
        let value = if enabled { "1" } else { "0" };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('karaoke_ball', ?1)",
            params![value],
        )
        .map_err(|e| e.to_string())?;
        Ok(enabled)
    }

    pub const DEFAULT_KARAOKE_BALL_COLOR: &'static str = "#ffc857";
    pub const DEFAULT_SUNG_HIGHLIGHT_COLOR: &'static str = "#ffdc8c";
    pub const DEFAULT_KARAOKE_BOUNCER: &'static str = "glow-orb";

    pub const DEFAULT_LYRICS_FONT: &'static str = "default";

    pub const DEFAULT_LYRICS_SOURCE: &'static str = "auto";
    const LYRICS_SOURCE_IDS: &[&str] = &["auto", "lrclib", "netease"];

    pub const DEFAULT_VISUALIZER_KIND: &'static str = "off";
    pub const DEFAULT_VISUALIZER_INTENSITY: f64 = 1.5;
    pub const DEFAULT_VISUALIZER_OPACITY: f64 = 0.8;
    const MAX_VISUALIZER_INTENSITY: f64 = 3.0;

    pub const VISUALIZER_KIND_IDS: &[&str] = &[
        "off",
        "spectrum",
        "waveform",
        "radial",
        "particles",
        "aurora",
        "starfield",
        "mirror",
        "tunnel",
        "matrix",
        "terrain",
    ];

    pub const LYRICS_FONT_IDS: &[&str] = &[
        "default",
        "rubik",
        "heebo",
        "assistant",
        "inter",
        "varela-round",
        "nunito",
        "secular-one",
        "bebas-neue",
        "anton",
        "frank-ruhl-libre",
        "playfair-display",
        "amatic-sc",
        "caveat",
        "pacifico",
        "press-start-2p",
        "monoton",
        "bungee",
    ];

    const BOUNCER_IDS: &[&str] = &[
        "glow-orb",
        "music-note",
        "spark",
        "heart",
        "glass-drop",
        "firefly",
        "comet",
        "eq-pulse",
        "mic",
        "paper-plane",
        "pixel",
        "crescent",
    ];

    fn read_setting_string(conn: &Connection, key: &str) -> Option<String> {
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .ok()
    }

    fn normalize_hex_color(raw: &str, fallback: &str) -> String {
        let s = raw.trim();
        let hex = if let Some(stripped) = s.strip_prefix('#') {
            stripped
        } else {
            s
        };
        if hex.len() == 6
            && hex.chars().all(|c| c.is_ascii_hexdigit())
        {
            return format!("#{}", hex.to_ascii_lowercase());
        }
        fallback.to_string()
    }

    pub fn get_karaoke_ball_color(&self) -> String {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_KARAOKE_BALL_COLOR.to_string(),
        };
        Self::read_setting_string(conn, "karaoke_ball_color")
            .map(|v| Self::normalize_hex_color(&v, Self::DEFAULT_KARAOKE_BALL_COLOR))
            .unwrap_or_else(|| Self::DEFAULT_KARAOKE_BALL_COLOR.to_string())
    }

    pub fn get_sung_highlight_color(&self) -> String {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_SUNG_HIGHLIGHT_COLOR.to_string(),
        };
        Self::read_setting_string(conn, "sung_highlight_color")
            .map(|v| Self::normalize_hex_color(&v, Self::DEFAULT_SUNG_HIGHLIGHT_COLOR))
            .unwrap_or_else(|| Self::DEFAULT_SUNG_HIGHLIGHT_COLOR.to_string())
    }

    pub fn set_karaoke_ball_color(&self, color: &str) -> Result<String, String> {
        let normalized = Self::normalize_hex_color(color, Self::DEFAULT_KARAOKE_BALL_COLOR);
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(normalized),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('karaoke_ball_color', ?1)",
            params![normalized],
        )
        .map_err(|e| e.to_string())?;
        Ok(normalized)
    }

    pub fn set_sung_highlight_color(&self, color: &str) -> Result<String, String> {
        let normalized = Self::normalize_hex_color(color, Self::DEFAULT_SUNG_HIGHLIGHT_COLOR);
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(normalized),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('sung_highlight_color', ?1)",
            params![normalized],
        )
        .map_err(|e| e.to_string())?;
        Ok(normalized)
    }

    pub fn get_karaoke_bouncer(&self) -> String {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_KARAOKE_BOUNCER.to_string(),
        };
        Self::read_setting_string(conn, "karaoke_bouncer")
            .map(|v| {
                let id = v.trim().to_lowercase();
                if Self::BOUNCER_IDS.contains(&id.as_str()) {
                    id
                } else {
                    Self::DEFAULT_KARAOKE_BOUNCER.to_string()
                }
            })
            .unwrap_or_else(|| Self::DEFAULT_KARAOKE_BOUNCER.to_string())
    }

    pub fn set_karaoke_bouncer(&self, bouncer_id: &str) -> Result<String, String> {
        let id = bouncer_id.trim().to_lowercase();
        let normalized = if Self::BOUNCER_IDS.contains(&id.as_str()) {
            id
        } else {
            Self::DEFAULT_KARAOKE_BOUNCER.to_string()
        };
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(normalized),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('karaoke_bouncer', ?1)",
            params![normalized],
        )
        .map_err(|e| e.to_string())?;
        Ok(normalized)
    }

    pub fn get_lyrics_font(&self) -> String {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_LYRICS_FONT.to_string(),
        };
        Self::read_setting_string(conn, "lyrics_font")
            .map(|v| {
                let id = v.trim().to_lowercase();
                if Self::LYRICS_FONT_IDS.contains(&id.as_str()) {
                    id
                } else {
                    Self::DEFAULT_LYRICS_FONT.to_string()
                }
            })
            .unwrap_or_else(|| Self::DEFAULT_LYRICS_FONT.to_string())
    }

    pub fn set_lyrics_font(&self, font_id: &str) -> Result<String, String> {
        let id = font_id.trim().to_lowercase();
        let normalized = if Self::LYRICS_FONT_IDS.contains(&id.as_str()) {
            id
        } else {
            Self::DEFAULT_LYRICS_FONT.to_string()
        };
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(normalized),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('lyrics_font', ?1)",
            params![normalized],
        )
        .map_err(|e| e.to_string())?;
        Ok(normalized)
    }

    pub fn get_visualizer_kind(&self) -> String {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_VISUALIZER_KIND.to_string(),
        };
        Self::read_setting_string(conn, "visualizer_kind")
            .map(|v| {
                let id = v.trim().to_lowercase();
                if Self::VISUALIZER_KIND_IDS.contains(&id.as_str()) {
                    id
                } else {
                    Self::DEFAULT_VISUALIZER_KIND.to_string()
                }
            })
            .unwrap_or_else(|| Self::DEFAULT_VISUALIZER_KIND.to_string())
    }

    pub fn set_visualizer_kind(&self, kind: &str) -> Result<String, String> {
        let id = kind.trim().to_lowercase();
        let normalized = if Self::VISUALIZER_KIND_IDS.contains(&id.as_str()) {
            id
        } else {
            Self::DEFAULT_VISUALIZER_KIND.to_string()
        };
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(normalized),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('visualizer_kind', ?1)",
            params![normalized],
        )
        .map_err(|e| e.to_string())?;
        Ok(normalized)
    }

    pub fn get_visualizer_intensity(&self) -> f64 {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_VISUALIZER_INTENSITY,
        };
        let raw = Self::read_setting_f64(conn, "visualizer_intensity")
            .unwrap_or(Self::DEFAULT_VISUALIZER_INTENSITY);
        let migrated = if raw <= 1.0 + 1e-6 {
            (raw * 1.75).min(Self::MAX_VISUALIZER_INTENSITY)
        } else {
            raw
        };
        migrated.clamp(0.0, Self::MAX_VISUALIZER_INTENSITY)
    }

    pub fn set_visualizer_intensity(&self, value: f64) -> Result<f64, String> {
        let clamped = value.clamp(0.0, Self::MAX_VISUALIZER_INTENSITY);
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(clamped),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('visualizer_intensity', ?1)",
            params![clamped.to_string()],
        )
        .map_err(|e| e.to_string())?;
        Ok(clamped)
    }

    pub fn get_visualizer_opacity(&self) -> f64 {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_VISUALIZER_OPACITY,
        };
        let raw = Self::read_setting_f64(conn, "visualizer_opacity")
            .unwrap_or(Self::DEFAULT_VISUALIZER_OPACITY);
        let migrated = if raw <= 0.551 {
            if (raw - 0.35).abs() < 0.02 {
                0.8
            } else {
                (raw / 0.55).clamp(0.0, 1.0)
            }
        } else {
            raw
        };
        migrated.clamp(0.0, 1.0)
    }

    pub fn set_visualizer_opacity(&self, value: f64) -> Result<f64, String> {
        let clamped = value.clamp(0.0, 1.0);
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(clamped),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('visualizer_opacity', ?1)",
            params![clamped.to_string()],
        )
        .map_err(|e| e.to_string())?;
        Ok(clamped)
    }

    pub fn reset_karaoke_colors(&self) -> Result<(), String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(()),
        };
        conn.execute(
            "DELETE FROM app_settings WHERE key IN ('karaoke_ball_color', 'sung_highlight_color')",
            [],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_overlay_opacity(&self, opacity: f64) -> Result<f64, String> {
        let clamped = opacity.clamp(0.0, 1.0);
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(clamped),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('background_opacity', ?1)",
            params![clamped.to_string()],
        )
        .map_err(|e| e.to_string())?;
        Ok(clamped)
    }

    pub fn add_sync_offset(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        delta_ms: i64,
    ) -> Result<i64, String> {
        let next = self.get_sync_offset(artist, track, album, duration_ms) + delta_ms;
        self.set_sync_offset(artist, track, album, duration_ms, next)?;
        Ok(next)
    }

    pub fn get_lyrics_source(&self) -> String {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Self::DEFAULT_LYRICS_SOURCE.to_string(),
        };
        Self::read_setting_string(conn, "lyrics_source")
            .map(|id| {
                let id = id.trim().to_lowercase();
                if Self::LYRICS_SOURCE_IDS.contains(&id.as_str()) {
                    id
                } else {
                    Self::DEFAULT_LYRICS_SOURCE.to_string()
                }
            })
            .unwrap_or_else(|| Self::DEFAULT_LYRICS_SOURCE.to_string())
    }

    pub fn set_lyrics_source(&self, mode: &str) -> Result<String, String> {
        let id = mode.trim().to_lowercase();
        let normalized = if Self::LYRICS_SOURCE_IDS.contains(&id.as_str()) {
            id
        } else {
            Self::DEFAULT_LYRICS_SOURCE.to_string()
        };
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(normalized),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('lyrics_source', ?1)",
            params![normalized],
        )
        .map_err(|e| e.to_string())?;
        Ok(normalized)
    }

    pub fn put(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        lyrics_source: &str,
        response: &LyricsResponse,
    ) -> Result<(), String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(()),
        };
        let key = Self::lyrics_cache_key(artist, track, album, duration_ms, lyrics_source);
        let ttl = match response.status.as_str() {
            "not_found" | "error" => NOT_FOUND_TTL_SECS,
            _ => FOUND_TTL_SECS,
        };
        let expires_at = Self::now() + ttl;
        let json = serde_json::to_string(response).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO lyrics_cache (cache_key, response_json, expires_at) VALUES (?1, ?2, ?3)",
            params![key, json, expires_at],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn cache_db_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("LyricGlass").join("lyrics-cache.sqlite3")
}

#[cfg(test)]
mod settings_tests {
    use super::LyricsCache;
    use rusqlite::Connection;

    fn open_memory() -> LyricsCache {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .expect("schema");
        LyricsCache { conn: Some(conn) }
    }

    #[test]
    fn sync_offset_v061_migrates_builtin_lag_out_of_stored() {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE sync_offset (cache_key TEXT PRIMARY KEY, offset_ms INTEGER NOT NULL);
             CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO sync_offset (cache_key, offset_ms) VALUES ('a|b|c|120', 600);
             INSERT INTO sync_offset (cache_key, offset_ms) VALUES ('x|y|z|200', 100);",
        )
        .expect("schema");
        LyricsCache::migrate_sync_offsets_v061(&conn).expect("migrate");
        let a: i64 = conn
            .query_row(
                "SELECT offset_ms FROM sync_offset WHERE cache_key = 'a|b|c|120'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let x: i64 = conn
            .query_row(
                "SELECT offset_ms FROM sync_offset WHERE cache_key = 'x|y|z|200'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(a, 0);
        assert_eq!(x, -500);
    }

    #[test]
    fn wallpaper_mode_defaults_false_and_persists() {
        let cache = open_memory();
        assert!(!cache.get_wallpaper_mode());
        cache.set_wallpaper_mode(true).expect("set");
        assert!(cache.get_wallpaper_mode());
        cache.set_wallpaper_mode(false).expect("clear");
        assert!(!cache.get_wallpaper_mode());
    }

    #[test]
    fn ensure_overlay_at_startup_clears_saved_wallpaper_mode() {
        let cache = open_memory();
        cache.set_wallpaper_mode(true).expect("set");
        assert!(cache.get_wallpaper_mode());
        cache.ensure_overlay_at_startup();
        assert!(!cache.get_wallpaper_mode());
    }
}