use serde::{Deserialize, Serialize};

const USER_AGENT: &str = "LyricGlass/0.2.2 https://github.com/shiper69-afk/LyricGlass";
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
    pub lrclib_duration_sec: Option<f64>,
    pub lrclib_track_name: Option<String>,
    pub lrclib_artist_name: Option<String>,
    pub lrclib_album_name: Option<String>,
    pub duration_delta_sec: Option<f64>,
    #[serde(default)]
    pub duration_warning: bool,
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
            lrclib_duration_sec: None,
            lrclib_track_name: None,
            lrclib_artist_name: None,
            lrclib_album_name: None,
            duration_delta_sec: None,
            duration_warning: false,
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
            lrclib_duration_sec: None,
            lrclib_track_name: None,
            lrclib_artist_name: None,
            lrclib_album_name: None,
            duration_delta_sec: None,
            duration_warning: false,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LrcRecord {
    id: i64,
    duration: Option<f64>,
    instrumental: Option<bool>,
    synced_lyrics: Option<String>,
    plain_lyrics: Option<String>,
    track_name: Option<String>,
    artist_name: Option<String>,
    album_name: Option<String>,
}

fn client() -> Result<reqwest::blocking::Client, LyricsResponse> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| LyricsResponse::err(format!("HTTP client: {e}")))
}

fn from_record(record: LrcRecord, source: &str, target_duration_sec: f64) -> LyricsResponse {
    let instrumental = record.instrumental.unwrap_or(false);
    let synced = record
        .synced_lyrics
        .filter(|s| !s.trim().is_empty());
    let plain = record.plain_lyrics.filter(|s| !s.trim().is_empty());

    let duration_delta_sec = record
        .duration
        .map(|d| d - target_duration_sec);
    let duration_warning = duration_delta_sec
        .map(|d| d.abs() > 3.0)
        .unwrap_or(false);

    let meta = |status: &str, message: String, synced_lrc: Option<String>, plain_lyrics: Option<String>, instrumental: bool| {
        LyricsResponse {
            status: status.into(),
            message,
            synced_lrc,
            plain_lyrics,
            instrumental,
            from_cache: false,
            lrclib_id: Some(record.id),
            lrclib_duration_sec: record.duration,
            lrclib_track_name: record.track_name.clone(),
            lrclib_artist_name: record.artist_name.clone(),
            lrclib_album_name: record.album_name.clone(),
            duration_delta_sec,
            duration_warning,
        }
    };

    if instrumental {
        return meta(
            "instrumental",
            "Instrumental track (LRCLIB)".into(),
            synced,
            plain,
            true,
        );
    }

    if let Some(lrc) = synced {
        return meta(
            "synced",
            format!("Synced lyrics ({source})"),
            Some(lrc),
            plain,
            false,
        );
    }

    if let Some(text) = plain {
        return meta(
            "plain",
            "Plain lyrics (untimed)".into(),
            None,
            Some(text),
            false,
        );
    }

    LyricsResponse::not_found()
}

fn duration_distance(record: &LrcRecord, target: f64) -> f64 {
    record
        .duration
        .map(|d| (d - target).abs())
        .unwrap_or(f64::MAX)
}

fn album_match_score(record_album: &Option<String>, target_album: &str) -> i32 {
    let Some(album) = record_album.as_ref() else {
        return 0;
    };
    let a = album.to_lowercase();
    let t = target_album.to_lowercase();
    if a == t {
        return 4;
    }
    if a.contains(&t) || t.contains(&a) {
        return 2;
    }
    0
}

fn has_synced_lyrics(record: &LrcRecord) -> bool {
    record
        .synced_lyrics
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty())
}

fn pick_best_record(
    records: Vec<LrcRecord>,
    target_duration_sec: f64,
    target_album: &str,
) -> Option<LrcRecord> {
    if records.is_empty() {
        return None;
    }
    let close: Vec<LrcRecord> = records
        .iter()
        .filter(|r| duration_distance(r, target_duration_sec) <= 2.0)
        .cloned()
        .collect();
    let pool = if close.is_empty() { records } else { close };
    pool.into_iter()
        .min_by(|a, b| {
            let da = duration_distance(a, target_duration_sec);
            let db = duration_distance(b, target_duration_sec);
            da.partial_cmp(&db)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    album_match_score(b.album_name.as_ref(), target_album)
                        .cmp(&album_match_score(a.album_name.as_ref(), target_album))
                })
                .then_with(|| {
                    has_synced_lyrics(b)
                        .cmp(&has_synced_lyrics(a))
                })
        })
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

    let duration_sec = (duration_ms as f64 / 1000.0).max(1.0);
    let target = duration_sec;

    let get_url = format!(
        "{BASE}/api/get?artist_name={}&track_name={}&album_name={}&duration={}",
        urlencoding::encode(artist.trim()),
        urlencoding::encode(track.trim()),
        urlencoding::encode(album.trim()),
        duration_sec.round() as u64
    );

    match client.get(&get_url).send() {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(record) = resp.json::<LrcRecord>() {
                return from_record(record, "LRCLIB get", target);
            }
            return LyricsResponse::err("Invalid LRCLIB get response");
        }
        Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => {}
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

    if let Some(record) = pick_best_record(records, target, album) {
        from_record(record, "LRCLIB search", target)
    } else {
        LyricsResponse::not_found()
    }
}

#[cfg(test)]
mod tests {
    use super::{duration_distance, pick_best_record, LrcRecord};

    #[test]
    fn picks_close_duration_first() {
        let records = vec![
            LrcRecord {
                id: 1,
                duration: Some(200.0),
                instrumental: None,
                synced_lyrics: Some("[00:01.00] a".into()),
                plain_lyrics: None,
                track_name: None,
                artist_name: None,
                album_name: None,
            },
            LrcRecord {
                id: 2,
                duration: Some(245.0),
                instrumental: None,
                synced_lyrics: Some("[00:01.00] b".into()),
                plain_lyrics: None,
                track_name: None,
                artist_name: None,
                album_name: None,
            },
        ];
        let best = pick_best_record(records, 243.0, "").unwrap();
        assert_eq!(best.id, 2);
        assert!(duration_distance(&best, 243.0) <= 2.0);
    }

    #[test]
    fn legends_never_die_prefers_full_length_over_short_edit() {
        let records = vec![
            LrcRecord {
                id: 11300315,
                duration: Some(179.0),
                instrumental: None,
                synced_lyrics: Some("[00:12.71] short".into()),
                plain_lyrics: None,
                track_name: Some("Legends Never Die".into()),
                artist_name: Some("Against The Current".into()),
                album_name: Some("Legends Never Die".into()),
            },
            LrcRecord {
                id: 7141512,
                duration: Some(235.0),
                instrumental: None,
                synced_lyrics: Some("[00:12.09] full".into()),
                plain_lyrics: None,
                track_name: Some("Legends Never Die".into()),
                artist_name: Some("Against The Current".into()),
                album_name: Some("2017 LoL WC".into()),
            },
        ];
        let best = pick_best_record(records, 234.0, "Legends Never Die").unwrap();
        assert_eq!(best.id, 7141512);
        assert!((best.duration.unwrap() - 234.0).abs() <= 2.0);
    }
}