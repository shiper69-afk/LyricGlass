use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResponse {
    pub status: String,
    pub message: String,
    pub synced_lrc: Option<String>,
    pub plain_lyrics: Option<String>,
    pub instrumental: bool,
    #[serde(default)]
    pub from_cache: bool,
    /// `lrclib-synced` | `lrclib-plain` | `shironet` | `instrumental` | empty
    #[serde(default)]
    pub lyrics_source: String,
    pub lrclib_id: Option<i64>,
    pub lrclib_duration_sec: Option<f64>,
    pub lrclib_track_name: Option<String>,
    pub lrclib_artist_name: Option<String>,
    pub lrclib_album_name: Option<String>,
    pub duration_delta_sec: Option<f64>,
    #[serde(default)]
    pub duration_warning: bool,
    pub shironet_wrkid: Option<u32>,
    pub shironet_prfid: Option<u32>,
    pub shironet_match_title: Option<String>,
    pub shironet_match_artist: Option<String>,
    pub shironet_page_url: Option<String>,
}

impl LyricsResponse {
    pub fn err(message: impl Into<String>) -> Self {
        Self::base("error", message.into())
    }

    pub fn not_found() -> Self {
        Self::base("not_found", "No lyrics found".into())
    }

    fn base(status: &str, message: String) -> Self {
        Self {
            status: status.into(),
            message,
            synced_lrc: None,
            plain_lyrics: None,
            instrumental: false,
            from_cache: false,
            lyrics_source: String::new(),
            lrclib_id: None,
            lrclib_duration_sec: None,
            lrclib_track_name: None,
            lrclib_artist_name: None,
            lrclib_album_name: None,
            duration_delta_sec: None,
            duration_warning: false,
            shironet_wrkid: None,
            shironet_prfid: None,
            shironet_match_title: None,
            shironet_match_artist: None,
            shironet_page_url: None,
        }
    }
}

pub fn looks_hebrew(s: &str) -> bool {
    s.chars().any(|c| ('\u{0590}'..='\u{05FF}').contains(&c))
}