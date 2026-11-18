//! Strict title/artist/duration matching for NetEase search hits.

const DURATION_TOL_MS: u64 = 3_000;

pub fn normalize_title(raw: &str) -> String {
    let s = strip_bracketed(raw);
    let s = strip_feat_suffix(&s);
    collapse_ws(&deunicode::deunicode(&s))
}

pub fn normalize_artist_token(raw: &str) -> String {
    let s = strip_feat_suffix(&strip_bracketed(raw));
    collapse_ws(&deunicode::deunicode(&s))
}

/// Split GSMTC artist string into comparable tokens.
pub fn spotify_artists(artist_field: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in artist_field.split(&[',', '/', ';', '|', '&'][..]) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let lower = part.to_lowercase();
        let chunk = if let Some(i) = lower.find(" feat.") {
            &part[..i]
        } else if let Some(i) = lower.find(" ft.") {
            &part[..i]
        } else {
            part
        };
        let n = normalize_artist_token(chunk);
        if !n.is_empty() && !out.contains(&n) {
            out.push(n);
        }
    }
    if out.is_empty() {
        let n = normalize_artist_token(artist_field);
        if !n.is_empty() {
            out.push(n);
        }
    }
    out
}

pub fn netease_artist_names(artists: &[String]) -> Vec<String> {
    artists
        .iter()
        .map(|a| normalize_artist_token(a))
        .filter(|a| !a.is_empty())
        .collect()
}

pub fn title_matches(expected_track: &str, candidate: &str) -> bool {
    normalize_title(expected_track) == normalize_title(candidate)
}

pub fn artist_matches(expected_artists: &[String], candidate_artists: &[String]) -> bool {
    if expected_artists.is_empty() || candidate_artists.is_empty() {
        return false;
    }
    let cand = netease_artist_names(candidate_artists);
    expected_artists
        .iter()
        .any(|e| cand.iter().any(|c| c == e || c.contains(e.as_str()) || e.contains(c.as_str())))
}

pub fn duration_matches(gsmtc_duration_ms: u64, candidate_ms: u64) -> bool {
    if gsmtc_duration_ms == 0 || candidate_ms == 0 {
        return false;
    }
    gsmtc_duration_ms.abs_diff(candidate_ms) <= DURATION_TOL_MS
}

/// Returns true only when title, any artist, and duration all match within tolerance.
pub fn strict_match(
    spotify_title: &str,
    spotify_artist: &str,
    gsmtc_duration_ms: u64,
    candidate_title: &str,
    candidate_artists: &[String],
    candidate_duration_ms: u64,
) -> bool {
    let artists = spotify_artists(spotify_artist);
    title_matches(spotify_title, candidate_title)
        && artist_matches(&artists, candidate_artists)
        && duration_matches(gsmtc_duration_ms, candidate_duration_ms)
}

fn strip_bracketed(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '(' | '（' | '[' => depth += 1,
            ')' | '）' | ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn strip_feat_suffix(s: &str) -> String {
    let lower = s.to_lowercase();
    for needle in [
        " feat.",
        " feat ",
        " ft.",
        " ft ",
        " featuring ",
        " - remaster",
        " remastered",
        " live at ",
        " live from ",
        " live version",
    ] {
        if let Some(i) = lower.find(needle) {
            return s[..i].trim().to_string();
        }
    }
    s.trim().to_string()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_title_same_artist_duration() {
        let artists = vec!["Taylor Swift".into()];
        assert!(!strict_match(
            "Anti-Hero",
            "Taylor Swift",
            200_000,
            "Blank Space",
            &artists,
            199_500,
        ));
    }

    #[test]
    fn accepts_strict_match() {
        let artists = vec!["Artist One".into()];
        assert!(strict_match(
            "Song Title (Live)",
            "Artist One",
            245_000,
            "Song Title",
            &artists,
            246_200,
        ));
    }

    #[test]
    fn rejects_duration_drift() {
        let artists = vec!["Artist".into()];
        assert!(!strict_match(
            "Track",
            "Artist",
            200_000,
            "Track",
            &artists,
            210_000,
        ));
    }

    #[test]
    fn normalize_strips_feat() {
        assert_eq!(normalize_title("Hello (feat. Someone)"), "hello");
    }
}