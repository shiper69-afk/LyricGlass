mod parse;

use super::response::LyricsResponse;
use crate::startup;
use parking_lot::Mutex;
use parse::{parse_lyrics_html, pick_best_search_hit, search_hits_from_html, SearchHit};
use std::sync::LazyLock;

const USER_AGENT: &str = "LyricGlass/0.5.0 https://github.com/shiper69-afk/LyricGlass";
const BASE: &str = "https://shironet.mako.co.il";
const SEARCH_PATH: &str = "/searchSongs?type=lyrics&q=";
const LYRICS_PATH: &str = "/artist?type=lyrics&lang=1";

static SHIRONET_HTTP: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn client() -> Result<reqwest::blocking::Client, LyricsResponse> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(14))
        .build()
        .map_err(|e| LyricsResponse::err(format!("Shironet HTTP client: {e}")))
}

fn decode_body(bytes: &[u8], content_type: Option<&str>) -> String {
    if content_type
        .map(|c| c.to_ascii_lowercase().contains("1255"))
        .unwrap_or(false)
    {
        if let Some(enc) = encoding_rs::Encoding::for_label(b"windows-1255") {
            let (cow, _, _) = enc.decode(bytes);
            return cow.into_owned();
        }
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    encoding_rs::WINDOWS_1255.decode(bytes).0.into_owned()
}

fn fetch_text(client: &reqwest::blocking::Client, url: &str) -> Result<String, LyricsResponse> {
    let resp = client
        .get(url)
        .send()
        .map_err(|e| LyricsResponse::err(format!("Shironet network: {e}")))?;
    if resp.status().is_success() {
        let ct = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let bytes = resp
            .bytes()
            .map_err(|e| LyricsResponse::err(format!("Shironet body: {e}")))?;
        let text = decode_body(&bytes, ct.as_deref());
        if text.contains("Radware Block Page") || text.contains("validate.perfdrive.com") {
            return Err(LyricsResponse::err(
                "Shironet blocked automated access (bot protection)",
            ));
        }
        Ok(text)
    } else {
        Err(LyricsResponse::err(format!(
            "Shironet HTTP {}",
            resp.status()
        )))
    }
}

fn search_queries(artist: &str, track: &str) -> Vec<String> {
    let mut out = Vec::new();
    let a = artist.trim();
    let t = track.trim();
    if !a.is_empty() && !t.is_empty() {
        out.push(format!("{a} {t}"));
        out.push(format!("{t} {a}"));
    }
    if !t.is_empty() {
        out.push(t.to_string());
    }
  if !a.is_empty() {
        out.push(a.to_string());
    }
    out.dedup();
    out
}

fn lyrics_url(hit: &SearchHit) -> String {
    format!(
        "{BASE}{LYRICS_PATH}&prfid={}&wrkid={}",
        hit.prfid, hit.wrkid
    )
}

pub fn fetch_from_network(artist: &str, track: &str) -> LyricsResponse {
    let _guard = SHIRONET_HTTP.lock();
    startup::step("lyrics: shironet lookup");

    let client = match client() {
        Ok(c) => c,
        Err(e) => return e,
    };

    let mut all_hits: Vec<SearchHit> = Vec::new();
    for q in search_queries(artist, track) {
        let url = format!(
            "{BASE}{SEARCH_PATH}{}",
            urlencoding::encode(&parse::encode_search_query(&q))
        );
        match fetch_text(&client, &url) {
            Ok(html) => {
                all_hits.extend(search_hits_from_html(&html));
            }
            Err(e) => {
                startup::step(&format!("shironet search fail: {}", e.message));
            }
        }
        if !all_hits.is_empty() {
            break;
        }
    }

    let Some(hit) = pick_best_search_hit(&all_hits, artist, track) else {
        startup::step("shironet: no search match");
        let mut nf = LyricsResponse::not_found();
        nf.lyrics_source = "shironet".into();
        nf.message = "No lyrics on Shironet".into();
        return nf;
    };

    let page_url = lyrics_url(&hit);
    let html = match fetch_text(&client, &page_url) {
        Ok(h) => h,
        Err(e) => return e,
    };

    match parse_lyrics_html(&html, artist, track) {
        Ok(parsed) => LyricsResponse {
            status: "plain".into(),
            message: "Plain lyrics (Shironet, untimed)".into(),
            synced_lrc: None,
            plain_lyrics: Some(parsed.lyrics),
            instrumental: false,
            from_cache: false,
            lyrics_source: "shironet".into(),
            lrclib_id: None,
            lrclib_duration_sec: None,
            lrclib_track_name: None,
            lrclib_artist_name: None,
            lrclib_album_name: None,
            duration_delta_sec: None,
            duration_warning: false,
            shironet_wrkid: Some(hit.wrkid),
            shironet_prfid: Some(hit.prfid),
            shironet_match_title: parsed.title.or_else(|| Some(hit.title.clone())),
            shironet_match_artist: parsed.artist.or_else(|| hit.artist.clone()),
            shironet_page_url: Some(page_url),
        },
        Err(reason) => {
            startup::step(&format!("shironet parse: {reason}"));
            let mut nf = LyricsResponse::not_found();
            nf.lyrics_source = "shironet".into();
            nf.message = format!("Shironet page found but lyrics parse failed: {reason}");
            nf.shironet_wrkid = Some(hit.wrkid);
            nf.shironet_prfid = Some(hit.prfid);
            nf.shironet_page_url = Some(page_url);
            nf
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse::{parse_lyrics_html, pick_best_search_hit, search_hits_from_html};

    #[test]
    fn parses_search_fixture() {
        let html = include_str!("fixtures/search_results.html");
        let hits = search_hits_from_html(html);
        assert!(!hits.is_empty());
        let best = pick_best_search_hit(&hits, "עומר אדם", "נפשי כלתה לחרוזים").unwrap();
        assert_eq!(best.wrkid, 3891);
        assert_eq!(best.prfid, 953);
    }

    #[test]
    fn parses_lyrics_fixture() {
        let html = include_str!("fixtures/lyrics_page.html");
        let parsed = parse_lyrics_html(html, "עומר אדם", "נפשי כלתה לחרוזים").unwrap();
        assert!(parsed.lyrics.contains("שורה ראשונה"));
        assert!(parsed.lyrics.lines().count() >= 3);
    }
}