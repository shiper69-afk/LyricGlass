//! Strict title/artist/duration matching for NetEase search hits.

pub const DURATION_TOL_STRICT_MS: u64 = 3_000;
pub const DURATION_TOL_EXACT_META_MS: u64 = 8_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    TitleMismatch,
    ArtistMismatch,
    DurationMismatch { spotify_ms: u64, candidate_ms: u64, tol_ms: u64 },
}

#[derive(Clone, Debug)]
pub struct CandidateInfo {
    pub id: i64,
    pub title: String,
    pub artists: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug)]
pub struct MatchResult {
    pub index: usize,
    pub candidate: CandidateInfo,
    pub duration_delta_ms: i64,
}

pub fn normalize_title(raw: &str) -> String {
    let s = strip_bracketed(raw);
    let s = strip_feat_suffix(&s);
    collapse_ws(&deunicode::deunicode(&s))
}

pub fn normalize_artist_token(raw: &str) -> String {
    let s = strip_feat_suffix(&strip_bracketed(raw));
    collapse_ws(&deunicode::deunicode(&s))
}

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
    expected_artists.iter().any(|e| {
        cand.iter()
            .any(|c| c == e || c.contains(e.as_str()) || e.contains(c.as_str()))
    })
}

pub fn duration_ok(gsmtc_duration_ms: u64, candidate_ms: u64, title_ok: bool, artist_ok: bool) -> bool {
    if gsmtc_duration_ms == 0 || candidate_ms == 0 {
        return false;
    }
    let delta = gsmtc_duration_ms.abs_diff(candidate_ms);
    if delta <= DURATION_TOL_STRICT_MS {
        return true;
    }
    title_ok && artist_ok && delta <= DURATION_TOL_EXACT_META_MS
}

pub fn reject_reason(
    spotify_title: &str,
    spotify_artist: &str,
    gsmtc_duration_ms: u64,
    candidate_title: &str,
    candidate_artists: &[String],
    candidate_duration_ms: u64,
) -> Option<RejectReason> {
    let artists = spotify_artists(spotify_artist);
    let title_ok = title_matches(spotify_title, candidate_title);
    let artist_ok = artist_matches(&artists, candidate_artists);
    if !title_ok {
        return Some(RejectReason::TitleMismatch);
    }
    if !artist_ok {
        return Some(RejectReason::ArtistMismatch);
    }
    if !duration_ok(gsmtc_duration_ms, candidate_duration_ms, title_ok, artist_ok) {
        return Some(RejectReason::DurationMismatch {
            spotify_ms: gsmtc_duration_ms,
            candidate_ms: candidate_duration_ms,
            tol_ms: if title_ok && artist_ok {
                DURATION_TOL_EXACT_META_MS
            } else {
                DURATION_TOL_STRICT_MS
            },
        });
    }
    None
}

/// Pick best strict match among candidates (closest duration when several qualify).
pub fn pick_best_match(
    candidates: &[CandidateInfo],
    spotify_title: &str,
    spotify_artist: &str,
    gsmtc_duration_ms: u64,
) -> Option<MatchResult> {
    let mut best: Option<MatchResult> = None;
    for (index, c) in candidates.iter().enumerate() {
        if reject_reason(
            spotify_title,
            spotify_artist,
            gsmtc_duration_ms,
            &c.title,
            &c.artists,
            c.duration_ms,
        )
        .is_some()
        {
            continue;
        }
        let delta = c.duration_ms as i64 - gsmtc_duration_ms as i64;
        let abs = delta.unsigned_abs();
        let replace = match &best {
            None => true,
            Some(b) => abs < b.duration_delta_ms.unsigned_abs(),
        };
        if replace {
            best = Some(MatchResult {
                index,
                candidate: c.clone(),
                duration_delta_ms: delta,
            });
        }
    }
    best
}

pub fn format_duration_mmss(ms: u64) -> String {
    let total_sec = ms / 1000;
    let m = total_sec / 60;
    let s = total_sec % 60;
    format!("{m}:{s:02}")
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
    fn stick_season_accepts_within_8s_exact_meta() {
        let candidates = vec![CandidateInfo {
            id: 1,
            title: "Stick Season".into(),
            artists: vec!["Noah Kahan".into()],
            duration_ms: 181_000, // Spotify ~182000
        }];
        let hit = pick_best_match(&candidates, "Stick Season", "Noah Kahan", 182_000);
        assert!(hit.is_some());
    }

    #[test]
    fn rejects_wrong_title_same_artist_duration() {
        let candidates = vec![CandidateInfo {
            id: 2,
            title: "Blank Space".into(),
            artists: vec!["Taylor Swift".into()],
            duration_ms: 199_500,
        }];
        assert!(pick_best_match(&candidates, "Anti-Hero", "Taylor Swift", 200_000).is_none());
    }

    #[test]
    fn rejects_duration_beyond_8s_even_when_title_artist_match() {
        let candidates = vec![CandidateInfo {
            id: 3,
            title: "Track".into(),
            artists: vec!["Artist".into()],
            duration_ms: 220_000,
        }];
        assert!(pick_best_match(&candidates, "Track", "Artist", 200_000).is_none());
    }
}