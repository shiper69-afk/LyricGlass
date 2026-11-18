mod eapi;
mod r#match;
mod yrc;

use super::response::LyricsResponse;
use super::source::netease_not_found;
use crate::startup;
use r#match::{
    format_duration_mmss, pick_best_match, reject_reason, CandidateInfo, RejectReason,
};
use serde::Deserialize;
use std::time::Duration;

pub(crate) const USER_AGENT: &str = "LyricGlass/0.9.1 https://github.com/shiper69-afk/LyricGlass";

const EAPI_SEARCH_PATH: &str = "/api/cloudsearch/pc";

#[derive(Deserialize)]
struct SearchWebResponse {
    result: Option<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    songs: Option<Vec<SearchSongWeb>>,
}

#[derive(Deserialize)]
struct SearchSongWeb {
    id: i64,
    name: String,
    #[serde(default)]
    artists: Vec<SearchArtist>,
    duration: u64,
}

#[derive(Deserialize)]
struct SearchArtist {
    name: String,
}

#[derive(Deserialize)]
struct EapiSearchResponse {
    #[serde(default)]
    code: i32,
    result: Option<EapiSearchResult>,
}

#[derive(Deserialize)]
struct EapiSearchResult {
    #[serde(default)]
    songs: Vec<SearchSongEapi>,
}

#[derive(Deserialize)]
struct SearchSongEapi {
    id: i64,
    name: String,
    #[serde(rename = "dt")]
    duration: u64,
    #[serde(default, rename = "ar")]
    artists: Vec<SearchArtist>,
}

#[derive(Deserialize)]
struct LyricV1Response {
    yrc: Option<LyricSection>,
    lrc: Option<LyricSection>,
}

#[derive(Deserialize)]
struct LyricSection {
    lyric: Option<String>,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))
}

fn trace_push(trace: &mut Vec<String>, line: impl Into<String>) {
    let s = line.into();
    startup::step(&format!("netease: {s}"));
    trace.push(s);
}

fn songs_from_web_json(body: &str) -> Result<Vec<CandidateInfo>, String> {
    if body.contains("\"abroad\":true") || body.contains(r#""result":"#) {
        return Err("web search returned encrypted abroad payload".into());
    }
    let parsed: SearchWebResponse =
        serde_json::from_str(body).map_err(|e| format!("web JSON: {e}"))?;
    let songs = parsed.result.and_then(|r| r.songs).unwrap_or_default();
    Ok(songs
        .into_iter()
        .map(|s| CandidateInfo {
            id: s.id,
            title: s.name,
            artists: s.artists.into_iter().map(|a| a.name).collect(),
            duration_ms: s.duration,
        })
        .collect())
}

fn search_web(client: &reqwest::blocking::Client, keyword: &str) -> Result<Vec<CandidateInfo>, String> {
    let url = format!(
        "https://music.163.com/api/search/get/web?s={}&type=1&limit=25&offset=0",
        urlencoding::encode(keyword)
    );
    let resp = client
        .get(&url)
        .header("Referer", "https://music.163.com/")
        .send()
        .map_err(|e| format!("web search HTTP: {e}"))?;
    let status = resp.status();
    let body = resp
        .text()
        .map_err(|e| format!("web search body: {e}"))?;
    if !status.is_success() {
        return Err(format!("web search HTTP {status}"));
    }
    songs_from_web_json(&body)
}

fn search_eapi(client: &reqwest::blocking::Client, keyword: &str) -> Result<Vec<CandidateInfo>, String> {
    let esc = keyword.replace('\\', "\\\\").replace('"', "\\\"");
    let body = format!(
        r#"{{"s":"{esc}","type":"1","limit":"25","offset":"0","total":"true"}}"#
    );
    let bytes = eapi::post_eapi(client, EAPI_SEARCH_PATH, "/cloudsearch/pc", &body)?;
    if bytes.is_empty() {
        return Err("eapi search empty response".into());
    }
    let parsed: EapiSearchResponse =
        serde_json::from_slice(&bytes).map_err(|e| format!("eapi search JSON: {e}"))?;
    if parsed.code != 200 && parsed.code != 0 {
        return Err(format!("eapi search code {}", parsed.code));
    }
    let songs = parsed.result.map(|r| r.songs).unwrap_or_default();
    Ok(songs
        .into_iter()
        .map(|s| CandidateInfo {
            id: s.id,
            title: s.name,
            artists: s.artists.into_iter().map(|a| a.name).collect(),
            duration_ms: s.duration,
        })
        .collect())
}

fn merge_search(
    client: &reqwest::blocking::Client,
    track: &str,
    artist: &str,
    trace: &mut Vec<String>,
) -> Vec<CandidateInfo> {
    let queries = [
        format!("{track} {artist}"),
        track.to_string(),
    ];
    let mut all = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for q in queries {
        trace_push(trace, format!("search query: {q:?}"));
        let mut try_eapi = false;
        match search_web(client, &q) {
            Ok(mut songs) => {
                trace_push(trace, format!("web search ok: {} candidates", songs.len()));
                if songs.is_empty() {
                    try_eapi = true;
                }
                for s in songs.drain(..) {
                    if seen_ids.insert(s.id) {
                        all.push(s);
                    }
                }
            }
            Err(e) => {
                trace_push(trace, format!("web search failed: {e}"));
                try_eapi = true;
            }
        }
        if try_eapi {
                match search_eapi(client, &q) {
                    Ok(mut songs) => {
                        trace_push(trace, format!("eapi search ok: {} candidates", songs.len()));
                        for s in songs.drain(..) {
                            if seen_ids.insert(s.id) {
                                all.push(s);
                            }
                        }
                    }
                    Err(e2) => trace_push(trace, format!("eapi search failed: {e2}")),
                }
        }
    }
    all.truncate(10);
    all
}

fn log_candidate_rejections(
    trace: &mut Vec<String>,
    candidates: &[CandidateInfo],
    title: &str,
    artist: &str,
    duration_ms: u64,
) {
    for (i, c) in candidates.iter().enumerate().take(10) {
        let reason = reject_reason(title, artist, duration_ms, &c.title, &c.artists, c.duration_ms);
        let line = match reason {
            None => format!(
                "#{} PASS {:?} / {:?} ({})",
                i,
                c.title,
                c.artists.join(", "),
                format_duration_mmss(c.duration_ms)
            ),
            Some(RejectReason::TitleMismatch) => format!(
                "#{} reject title: {:?} vs expected {:?}",
                i, c.title, title
            ),
            Some(RejectReason::ArtistMismatch) => format!(
                "#{} reject artist: {:?} / {:?}",
                i,
                c.artists.join(", "),
                artist
            ),
            Some(RejectReason::DurationMismatch {
                spotify_ms,
                candidate_ms,
                ..
            }) => format!(
                "#{} reject duration: Spotify {} vs NetEase {} (cand {:?})",
                i,
                format_duration_mmss(spotify_ms),
                format_duration_mmss(candidate_ms),
                c.title
            ),
        };
        trace_push(trace, line);
    }
}

fn fetch_lyric_v1(
    client: &reqwest::blocking::Client,
    song_id: i64,
    trace: &mut Vec<String>,
) -> Result<LyricV1Response, String> {
    let body = format!(
        r#"{{"id":"{song_id}","cp":false,"tv":0,"lv":0,"rv":0,"kv":0,"yv":0,"ytv":0,"yrv":0}}"#
    );
    let bytes = eapi::post_eapi(client, eapi::LYRIC_V1_PATH, "/song/lyric/v1", &body)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("lyric v1 JSON: {e}"))
}

fn response_meta(
    status: &str,
    message: String,
    lyrics_source: &str,
    synced: Option<String>,
    plain: Option<String>,
    song: &CandidateInfo,
    song_id: i64,
    duration_ms: u64,
    word_level: bool,
    trace: Vec<String>,
) -> LyricsResponse {
    let delta = (song.duration_ms as f64 / 1000.0) - (duration_ms as f64 / 1000.0);
    LyricsResponse {
        status: status.into(),
        message,
        synced_lrc: synced,
        plain_lyrics: plain,
        instrumental: false,
        from_cache: false,
        lyrics_source: lyrics_source.into(),
        word_level,
        lrclib_id: None,
        lrclib_duration_sec: None,
        lrclib_track_name: None,
        lrclib_artist_name: None,
        lrclib_album_name: None,
        duration_delta_sec: Some(delta),
        duration_warning: delta.abs() > 3.0,
        shironet_wrkid: None,
        shironet_prfid: None,
        shironet_match_title: None,
        shironet_match_artist: None,
        shironet_page_url: None,
        netease_song_id: Some(song_id),
        netease_match_title: Some(song.title.clone()),
        netease_match_artist: Some(song.artists.first().cloned().unwrap_or_default()),
        netease_duration_ms: Some(song.duration_ms),
        lookup_trace: trace,
    }
}

/// Network fetch with diagnostic trace in `lookup_trace`.
pub fn fetch_from_network(
    artist: &str,
    track: &str,
    _album: &str,
    duration_ms: u64,
) -> LyricsResponse {
    let mut trace = Vec::new();
    if duration_ms == 0 {
        return netease_not_found("No lyrics on NetEase (duration unknown)", trace);
    }

    let client = match client() {
        Ok(c) => c,
        Err(e) => {
            trace_push(&mut trace, format!("client error: {e}"));
            return netease_not_found(
                format!("NetEase unavailable (request failed: {e})"),
                trace,
            );
        }
    };

    let candidates = merge_search(&client, track, artist, &mut trace);
    trace_push(
        &mut trace,
        format!("unique candidates (top 10): {}", candidates.len()),
    );
    if candidates.is_empty() {
        return netease_not_found(
            "NetEase unavailable (search returned no candidates)",
            trace,
        );
    }

    log_candidate_rejections(&mut trace, &candidates, track, artist, duration_ms);

    let Some(best) = pick_best_match(&candidates, track, artist, duration_ms) else {
        let best_title = candidates.first().map(|c| c.title.as_str()).unwrap_or("?");
        let best_dur = candidates
            .first()
            .map(|c| format_duration_mmss(c.duration_ms))
            .unwrap_or_else(|| "?".into());
        let msg = if let Some(c) = candidates.first() {
            if let Some(RejectReason::DurationMismatch {
                spotify_ms,
                candidate_ms,
                ..
            }) = reject_reason(track, artist, duration_ms, &c.title, &c.artists, c.duration_ms)
            {
                format!(
                    "No lyrics on NetEase (length mismatch: {} vs {})",
                    format_duration_mmss(spotify_ms),
                    format_duration_mmss(candidate_ms)
                )
            } else {
                format!("No lyrics on NetEase (no match; best candidate: {best_title} {best_dur})")
            }
        } else {
            "No lyrics on NetEase (no match)".into()
        };
        return netease_not_found(msg, trace);
    };

    trace_push(
        &mut trace,
        format!(
            "matched: {:?} / {:?} id pending lyric fetch",
            best.candidate.title,
            best.candidate.artists.join(", ")
        ),
    );

    let song = &candidates[best.index];
    let song_id = song.id;
    trace_push(
        &mut trace,
        format!("selected song id={song_id} delta_ms={}", best.duration_delta_ms),
    );

    let lyric = match fetch_lyric_v1(&client, song_id, &mut trace) {
        Ok(l) => l,
        Err(e) => {
            trace_push(&mut trace, format!("lyric v1 failed: {e}"));
            return netease_not_found(format!("NetEase unavailable (request failed: {e})"), trace);
        }
    };

    let has_yrc = lyric
        .yrc
        .as_ref()
        .and_then(|s| s.lyric.as_ref())
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false);
    trace_push(&mut trace, format!("YRC present: {}", if has_yrc { "yes" } else { "no" }));

    if let Some(yrc) = lyric.yrc.and_then(|s| s.lyric).filter(|t| !t.trim().is_empty()) {
        if let Some(enhanced) = yrc::yrc_to_enhanced_lrc(&yrc) {
            return response_meta(
                "synced",
                "Synced lyrics (NetEase word-level)".into(),
                "netease-yrc",
                Some(enhanced),
                None,
                song,
                song_id,
                duration_ms,
                true,
                trace,
            );
        }
        trace_push(&mut trace, "YRC parse produced no lines".into());
    }

    if let Some(lrc) = lyric
        .lrc
        .and_then(|s| s.lyric)
        .filter(|t| !t.trim().is_empty() && t.contains('['))
    {
        return response_meta(
            "synced",
            "Synced lyrics (NetEase line-level)".into(),
            "netease-synced",
            Some(lrc),
            None,
            song,
            song_id,
            duration_ms,
            false,
            trace,
        );
    }

    netease_not_found("No lyrics on NetEase (track matched but no lyric payload)", trace)
}

#[cfg(test)]
mod tests {
    use super::r#match::pick_best_match;
    use super::r#match::CandidateInfo;

    #[test]
    fn stick_season_like_match() {
        let candidates = vec![CandidateInfo {
            id: 1,
            title: "Stick Season".into(),
            artists: vec!["Noah Kahan".into()],
            duration_ms: 182_293,
        }];
        assert!(pick_best_match(&candidates, "Stick Season", "Noah Kahan", 182_000).is_some());
    }
}

#[cfg(test)]
mod live {
    use super::*;

  /// Live NetEase API (disabled in CI). Run: cargo test stick_season_live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn stick_season_live_netease_search() {
        let client = client().expect("client");
        let mut trace = Vec::new();
        let songs = merge_search(&client, "Stick Season", "Noah Kahan", &mut trace);
        eprintln!("trace:\n{}", trace.join("\n"));
        eprintln!("candidates: {:?}", songs.iter().take(5).collect::<Vec<_>>());
        let hit = pick_best_match(&songs, "Stick Season", "Noah Kahan", 182_000);
        eprintln!("match: {:?}", hit);
        assert!(!songs.is_empty(), "expected eapi search to return Stick Season");
    }
}