use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "LyricGlass/0.2.0 https://github.com/shiper69-afk/LyricGlass";
const BASE: &str = "https://lrclib.net";

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
    pub lrclib_id: Option<i64>,
}

impl LyricsResponse {
    fn err(message: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            message: message.into(),
            synced_lrc: None,
            plain_lyrics: None,
            instrumental: false,
            from_cache: false,
            lrclib_id: None,
        }
    }

    fn not_found() -> Self {
        Self {
            status: "not_found".into(),
            message: "No lyrics found".into(),
            synced_lrc: None,
            plain_lyrics: None,
            instrumental: false,
            from_cache: false,
            lrclib_id: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcRecord {
    id: i64,
    duration: Option<f64>,
    instrumental: Option<bool>,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
}

fn client() -> Result<reqwest::blocking::Client, LyricsResponse> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| LyricsResponse::err(format!("HTTP client: {e}")))
}

fn from_record(record: LrcRecord, source: &str) -> LyricsResponse {
    let instrumental = record.instrumental.unwrap_or(false);
    let synced = record
        .synced_lyrics
        .filter(|s| !s.trim().is_empty());
    let plain = record.plain_lyrics.filter(|s| !s.trim().is_empty());

    if instrumental {
        return LyricsResponse {
            status: "instrumental".into(),
            message: "Instrumental track (LRCLIB)".into(),
            synced_lrc: synced,
            plain_lyrics: plain,
            instrumental: true,
            from_cache: false,
            lrclib_id: Some(record.id),
        };
    }

    if let Some(lrc) = synced {
        return LyricsResponse {
            status: "synced".into(),
            message: format!("Synced lyrics ({source})"),
            synced_lrc: Some(lrc),
            plain_lyrics: plain,
            instrumental: false,
            from_cache: false,
            lrclib_id: Some(record.id),
        };
    }

    if let Some(text) = plain {
        return LyricsResponse {
            status: "plain".into(),
            message: "Plain lyrics (untimed)".into(),
            synced_lrc: None,
            plain_lyrics: Some(text),
            instrumental: false,
            from_cache: false,
            lrclib_id: Some(record.id),
        };
    }

    LyricsResponse::not_found()
}

pub fn fetch_from_network(
    artist: &str,
    track: &str,
    album: &str,
    duration_ms: u64,
) -> LyricsResponse {
    let client = match client() {
        Ok(c) => c,
        Err(e) => return e,
    };

    let duration_sec = (duration_ms / 1000).max(1);

    let get_url = format!(
        "{BASE}/api/get?artist_name={}&track_name={}&album_name={}&duration={duration_sec}",
        urlencoding::encode(artist.trim()),
        urlencoding::encode(track.trim()),
        urlencoding::encode(album.trim()),
    );

    match client.get(&get_url).send() {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(record) = resp.json::<LrcRecord>() {
                return from_record(record, "LRCLIB get");
            }
            return LyricsResponse::err("Invalid LRCLIB get response");
        }
        Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => {
            // fall through to search
        }
        Ok(resp) => {
            return LyricsResponse::err(format!(
                "LRCLIB get failed: HTTP {}",
                resp.status()
            ));
        }
        Err(e) => return LyricsResponse::err(format!("Network error: {e}")),
    }

    let search_url = format!(
        "{BASE}/api/search?track_name={}&artist_name={}&album_name={}",
        urlencoding::encode(track.trim()),
        urlencoding::encode(artist.trim()),
        urlencoding::encode(album.trim()),
    );

    let resp = match client.get(&search_url).send() {
        Ok(r) => r,
        Err(e) => return LyricsResponse::err(format!("Search network error: {e}")),
    };

    if !resp.status().is_success() {
        return LyricsResponse::err(format!("LRCLIB search: HTTP {}", resp.status()));
    }

    let records: Vec<LrcRecord> = match resp.json() {
        Ok(v) => v,
        Err(e) => return LyricsResponse::err(format!("Search JSON error: {e}")),
    };

    if records.is_empty() {
        return LyricsResponse::not_found();
    }

    let target = duration_sec as f64;
    let mut best = records.into_iter().min_by(|a, b| {
        let da = a
            .duration
            .map(|d| (d - target).abs())
            .unwrap_or(f64::MAX);
        let db = b
            .duration
            .map(|d| (d - target).abs())
            .unwrap_or(f64::MAX);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });

    if let Some(record) = best.take() {
        from_record(record, "LRCLIB search")
    } else {
        LyricsResponse::not_found()
    }
}

#[cfg(test)]
mod tests {
    use super::LyricsResponse;

    #[test]
    fn not_found_message() {
        let r = LyricsResponse::not_found();
        assert_eq!(r.status, "not_found");
        assert!(r.message.contains("No lyrics"));
    }
}