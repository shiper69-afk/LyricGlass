use super::lrclib::LyricsResponse;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const FOUND_TTL_SECS: i64 = 7 * 24 * 3600;
const NOT_FOUND_TTL_SECS: i64 = 6 * 3600;

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
        Ok(Self { conn: Some(conn) })
    }

    pub fn disabled() -> Self {
        Self { conn: None }
    }

    fn key(artist: &str, track: &str, album: &str, duration_ms: u64) -> String {
        let norm = |s: &str| s.trim().to_lowercase();
        let duration_sec = duration_ms / 1000;
        format!(
            "{}|{}|{}|{duration_sec}",
            norm(artist),
            norm(track),
            norm(album)
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
    ) -> Option<LyricsResponse> {
        let conn = self.conn.as_ref()?;
        let key = Self::key(artist, track, album, duration_ms);
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
        let key = Self::key(artist, track, album, duration_ms);
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
        let key = Self::key(artist, track, album, duration_ms);
        conn.execute(
            "INSERT OR REPLACE INTO sync_offset (cache_key, offset_ms) VALUES (?1, ?2)",
            params![key, offset_ms],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_overlay_opacity(&self) -> f64 {
        let conn = match &self.conn {
            Some(c) => c,
            None => return 0.62,
        };
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'overlay_opacity'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|o: &f64| (0.30..=1.0).contains(o))
        .unwrap_or(0.62)
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

    pub fn set_overlay_opacity(&self, opacity: f64) -> Result<f64, String> {
        let clamped = opacity.clamp(0.30, 1.0);
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(clamped),
        };
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('overlay_opacity', ?1)",
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

    pub fn put(
        &self,
        artist: &str,
        track: &str,
        album: &str,
        duration_ms: u64,
        response: &LyricsResponse,
    ) -> Result<(), String> {
        let conn = match &self.conn {
            Some(c) => c,
            None => return Ok(()),
        };
        let key = Self::key(artist, track, album, duration_ms);
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