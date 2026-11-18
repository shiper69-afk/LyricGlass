//! User-facing not-found messages and source-mode routing rules.

use super::response::{looks_hebrew, LyricsResponse};

pub fn should_try_shironet_auto(
    mode: &str,
    artist: &str,
    track: &str,
    primary: &LyricsResponse,
) -> bool {
    if mode != "auto" {
        return false;
    }
    if matches!(primary.status.as_str(), "synced" | "plain" | "instrumental") {
        return false;
    }
    looks_hebrew(artist) || looks_hebrew(track) || primary.status == "not_found"
}

pub fn netease_not_found(message: impl Into<String>, trace: Vec<String>) -> LyricsResponse {
    let mut r = LyricsResponse::base("not_found", message.into());
    r.lyrics_source = "netease".into();
    r.lookup_trace = trace;
    r
}

pub fn lrclib_not_found(message: impl Into<String>, trace: Vec<String>) -> LyricsResponse {
    let mut r = LyricsResponse::base("not_found", message.into());
    r.lyrics_source = "lrclib".into();
    r.lookup_trace = trace;
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shironet_only_in_auto() {
        let nf = LyricsResponse::not_found();
        assert!(!should_try_shironet_auto("netease", "Noah", "Stick", &nf));
        assert!(!should_try_shironet_auto("lrclib", "Noah", "Stick", &nf));
        assert!(should_try_shironet_auto("auto", "Noah", "Stick", &nf));
    }

    #[test]
    fn auto_skips_shironet_when_lrclib_has_lyrics() {
        let mut ok = LyricsResponse::not_found();
        ok.status = "synced".into();
        assert!(!should_try_shironet_auto("auto", "x", "y", &ok));
    }

    #[test]
    fn netease_not_found_message_prefix() {
        let r = netease_not_found("No lyrics on NetEase (no match)", vec![]);
        assert_eq!(r.status, "not_found");
        assert!(r.message.contains("NetEase"));
        assert_eq!(r.lyrics_source, "netease");
    }
}