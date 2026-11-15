use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub prfid: u32,
    pub wrkid: u32,
    pub title: String,
    pub artist: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ParsedLyrics {
    pub lyrics: String,
    pub title: Option<String>,
    pub artist: Option<String>,
}

static SEARCH_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)<a\s+href="/artist\?type=lyrics&lang=1&prfid=(\d+)&wrkid=(\d+)"[^>]*class="search_link_name(?:_big)?"[^>]*>([^<]*)</a>"#,
    )
    .unwrap()
});

static TITLE_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<title>([^<]*)</title>").unwrap()
});

static TITLE_SPAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<span class="artist_song_name_txt">([^<]*)</span>"#).unwrap()
});

static ARTIST_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<a class="artist_singer_title"[^>]*>([^<]*)</a>"#,
    )
    .unwrap()
});

static LYRICS_SPAN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<span class="artist_lyrics_text">(.*?)</span>"#).unwrap()
});

pub fn encode_search_query(q: &str) -> String {
    let mut out = q.to_string();
    for (from, to) in [
        ("(", ""),
        (")", ""),
        ("#", ""),
        ("/", ""),
        ("&amp;", "&"),
    ] {
        out = out.replace(from, to);
    }
    out.trim().to_string()
}

pub fn normalize_match_text(s: &str) -> String {
    let mut t = String::new();
    for ch in s.chars() {
        if ('\u{0591}'..='\u{05C7}').contains(&ch) {
            continue;
        }
        if ch.is_ascii() {
            let c = ch.to_ascii_lowercase();
            if c.is_alphanumeric() || c == ' ' {
                t.push(c);
            } else {
                t.push(' ');
            }
        } else if !ch.is_whitespace() {
            t.push(ch);
        } else {
            t.push(' ');
        }
    }
    let collapsed: Vec<&str> = t.split_whitespace().collect();
    let joined = collapsed.join(" ");
    strip_feat_clauses(&joined)
}

fn strip_feat_clauses(s: &str) -> String {
    let lower = s.to_lowercase();
    let markers = ["feat", "featuring", "ft", "with"];
    for m in markers {
        if let Some(i) = lower.find(m) {
            return s[..i].trim().to_string();
        }
    }
    s.trim().to_string()
}

pub fn match_score(want: &str, got: &str) -> i32 {
    let w = normalize_match_text(want);
    let g = normalize_match_text(got);
    if w.is_empty() || g.is_empty() {
        return 0;
    }
    if w == g {
        return 100;
    }
    if g.contains(&w) || w.contains(&g) {
        return 70;
    }
    let w_tokens: Vec<&str> = w.split_whitespace().collect();
    let g_tokens: Vec<&str> = g.split_whitespace().collect();
    let mut hits = 0;
    for wt in &w_tokens {
        if g_tokens.iter().any(|gt| gt == wt || gt.contains(wt) || wt.contains(gt)) {
            hits += 1;
        }
    }
    if hits == 0 {
        return 0;
    }
    (hits * 50) / w_tokens.len().max(1) as i32
}

pub fn search_hits_from_html(html: &str) -> Vec<SearchHit> {
    SEARCH_LINK
        .captures_iter(html)
        .filter_map(|c| {
            let prfid: u32 = c.get(1)?.as_str().parse().ok()?;
            let wrkid: u32 = c.get(2)?.as_str().parse().ok()?;
            let title = decode_entities(c.get(3)?.as_str().trim());
            if prfid == 0 || wrkid == 0 || title.is_empty() {
                return None;
            }
            Some(SearchHit {
                prfid,
                wrkid,
                title,
                artist: None,
            })
        })
        .collect()
}

pub fn pick_best_search_hit(
    hits: &[SearchHit],
    artist: &str,
    track: &str,
) -> Option<SearchHit> {
    hits.iter()
        .map(|h| {
            let title_score = match_score(track, &h.title);
            let artist_score = h
                .artist
                .as_deref()
                .map(|a| match_score(artist, a))
                .unwrap_or(0);
            let score = title_score * 2 + artist_score;
            (score, h)
        })
        .filter(|(s, _)| *s > 0)
        .max_by_key(|(s, _)| *s)
        .map(|(_, h)| h.clone())
        .or_else(|| hits.first().cloned())
}

pub fn parse_lyrics_html(
    html: &str,
    want_artist: &str,
    want_track: &str,
) -> Result<ParsedLyrics, String> {
    if html.contains("artist_lyrics_text") == false && html.contains("Radware Block Page") {
        return Err("unexpected page layout".into());
    }

    let title = TITLE_SPAN
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| decode_entities(m.as_str().trim()))
        .filter(|s| !s.is_empty());

    let artist = ARTIST_LINK
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| decode_entities(m.as_str().trim()))
        .filter(|s| !s.is_empty());

    let title_from_head = TITLE_TAG.captures(html).and_then(|c| c.get(1)).map(|m| {
        let raw = decode_entities(m.as_str());
        raw.split('-')
            .next()
            .unwrap_or(&raw)
            .trim()
            .to_string()
    });

    let final_title = title.or(title_from_head);

    let artist_ok = artist
        .as_deref()
        .map(|a| match_score(want_artist, a) >= 40)
        .unwrap_or(true);
    let title_ok = final_title
        .as_deref()
        .map(|t| match_score(want_track, t) >= 40)
        .unwrap_or(true);

    if !artist_ok && !title_ok {
        return Err("artist/title validation failed".into());
    }

    let lyrics_raw = LYRICS_SPAN
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| "lyrics span not found".to_string())?;

    let lyrics = clean_lyrics_html(lyrics_raw);
    if lyrics.len() < 2 {
        return Err("empty lyrics body".into());
    }
    if lyrics.contains("<a ") || lyrics.contains("href=") {
        return Err("lyrics contain raw HTML links".into());
    }

    Ok(ParsedLyrics {
        lyrics,
        title: final_title,
        artist,
    })
}

fn clean_lyrics_html(raw: &str) -> String {
    let with_newlines = raw
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<BR>", "\n");
    let no_tags = regex::Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&with_newlines, "");
    decode_entities(&no_tags)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode_entities(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_niqqud_and_feat() {
        let n = normalize_match_text("Hello (feat. Someone)");
        assert!(!n.contains("feat"));
        let h = normalize_match_text("שָׁלוֹם");
        assert_eq!(h, "שלום");
    }
}