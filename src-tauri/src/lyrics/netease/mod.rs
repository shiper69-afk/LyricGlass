mod eapi;
mod r#match;
mod yrc;

use super::response::LyricsResponse;
use crate::startup;
pub(crate) use r#match::strict_match;
use serde::Deserialize;
use std::time::Duration;

pub(crate) const USER_AGENT: &str = "LyricGlass/0.9.0 https://github.com/shiper69-afk/LyricGlass";

#[derive(Deserialize)]
struct SearchWebResponse {
    result: Option<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    songs: Option<Vec<SearchSong>>,
}

#[derive(Deserialize)]
struct SearchSong {
    id: i64,
    name: String,
  artists: Vec<SearchArtist>,
    duration: u64,
}

#[derive(Deserialize)]
struct SearchArtist {
    name: String,
}

#[derive(Deserialize)]
struct LyricV1Response {
    yrc: Option<LyricSection>,
    lrc: Option<LyricSection>,
}

#[derive(Deserialize)]
struct LyricSection {
    lyric: Option<String>,
    #[serde(default)]
    version: i64,
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| format!("HTTP client: {e}"))
}

fn search_songs(
    client: &reqwest::blocking::Client,
    keyword: &str,
) -> Result<Vec<SearchSong>, String> {
    let url = format!(
        "https://music.163.com/api/search/get/web?s={}&type=1&limit=25&offset=0",
        urlencoding::encode(keyword)
    );
    let resp = client
        .get(&url)
        .header("Referer", "https://music.163.com/")
        .send()
        .map_err(|e| format!("netease search: {e}"))?;
    let parsed: SearchWebResponse = resp.json().map_err(|e| format!("netease search JSON: {e}"))?;
    Ok(parsed
        .result
        .and_then(|r| r.songs)
        .unwrap_or_default())
}

fn pick_strict_hit<'a>(
    songs: &'a [SearchSong],
    title: &str,
    artist: &str,
    duration_ms: u64,
) -> Option<&'a SearchSong> {
    for song in songs {
        let names: Vec<String> = song.artists.iter().map(|a| a.name.clone()).collect();
        if strict_match(title, artist, duration_ms, &song.name, &names, song.duration) {
            return Some(song);
        }
    }
    None
}

fn fetch_lyric_v1(client: &reqwest::blocking::Client, song_id: i64) -> Result<LyricV1Response, String> {
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
    song: &SearchSong,
    duration_ms: u64,
    word_level: bool,
) -> LyricsResponse {
    let delta = (song.duration as f64 / 1000.0) - (duration_ms as f64 / 1000.0);
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
        netease_song_id: Some(song.id),
        netease_match_title: Some(song.name.clone()),
        netease_match_artist: Some(
            song.artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
        ),
        netease_duration_ms: Some(song.duration),
    }
}

/// Network fetch; returns `not_found` when no strict match or no lyrics (no panic).
pub fn fetch_from_network(
    artist: &str,
    track: &str,
    _album: &str,
    duration_ms: u64,
) -> LyricsResponse {
    if duration_ms == 0 {
        return LyricsResponse::not_found();
    }

    let client = match client() {
        Ok(c) => c,
        Err(e) => {
            startup::step(&format!("netease: {e}"));
            return LyricsResponse::not_found();
        }
    };

    let keyword = format!("{track} {artist}");
    startup::step("lyrics: netease search");
    let songs = match search_songs(&client, &keyword) {
        Ok(s) => s,
        Err(e) => {
            startup::step(&format!("netease: {e}"));
            return LyricsResponse::not_found();
        }
    };

    let Some(hit) = pick_strict_hit(&songs, track, artist, duration_ms) else {
        startup::step("netease: no strict search match");
        return LyricsResponse::not_found();
    };

    startup::step(&format!("netease: matched id={}", hit.id));
    let lyric = match fetch_lyric_v1(&client, hit.id) {
        Ok(l) => l,
        Err(e) => {
            startup::step(&format!("netease: {e}"));
            return LyricsResponse::not_found();
        }
    };

    if let Some(yrc) = lyric
        .yrc
        .and_then(|s| s.lyric)
        .filter(|t| !t.trim().is_empty())
    {
        if let Some(enhanced) = yrc::yrc_to_enhanced_lrc(&yrc) {
            return response_meta(
                "synced",
                "Synced lyrics (NetEase word-level)".into(),
                "netease-yrc",
                Some(enhanced),
                None,
                hit,
                duration_ms,
                true,
            );
        }
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
            hit,
            duration_ms,
            false,
        );
    }

    LyricsResponse::not_found()
}

#[cfg(test)]
mod tests {
    use super::r#match::strict_match;

    #[test]
    fn wrong_song_rejected() {
        let artists = vec!["Adele".into()];
        assert!(!strict_match(
            "Hello",
            "Adele",
            295_000,
            "Someone Like You",
            &artists,
            295_500,
        ));
    }
}