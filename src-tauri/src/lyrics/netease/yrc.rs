//! Parse NetEase YRC word-level lyrics into enhanced LRC for the frontend.

use std::fmt::Write;

pub fn is_metadata_line(line: &str) -> bool {
    line.trim_start().starts_with('{')
}

/// Parse YRC body into enhanced LRC (`[line]` + `<tag>word` segments).
pub fn yrc_to_enhanced_lrc(yrc: &str) -> Option<String> {
    let mut out = String::new();
    let mut lines_written = 0;

    for raw in yrc.lines() {
        let line = raw.trim();
        if line.is_empty() || is_metadata_line(line) {
            continue;
        }
        let Some(lrc_line) = parse_yrc_line(line) else {
            continue;
        };
        writeln!(out, "{lrc_line}").ok()?;
        lines_written += 1;
    }

    if lines_written == 0 {
        None
    } else {
        Some(out)
    }
}

fn parse_yrc_line(line: &str) -> Option<String> {
    let comma = line.find(',')?;
    let line_start: i64 = line[..comma].trim().parse().ok()?;
    let after_comma = &line[comma + 1..];
    let dur_end = after_comma
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_comma.len());
    if dur_end == 0 {
        return None;
    }
    let _line_dur: i64 = after_comma[..dur_end].parse().ok()?;
    let mut rest = after_comma[dur_end..].trim_start();

    let line_tag = ms_to_line_tag(line_start);
    let mut raw_enhanced = String::new();

    if let Some(paren) = rest.find('(') {
        if paren > 0 {
            let prefix = rest[..paren].trim();
            if !prefix.is_empty() {
                write!(raw_enhanced, "{}{prefix}", ms_to_inline_tag(line_start)).ok()?;
            }
            rest = &rest[paren..];
        }
    } else if !rest.is_empty() {
        write!(raw_enhanced, "{}{rest}", ms_to_inline_tag(line_start)).ok()?;
        return Some(format!("{line_tag}{raw_enhanced}"));
    }

    while let Some((w_start, _w_dur, text, remaining)) = next_yrc_word(rest) {
        if !text.is_empty() {
            write!(raw_enhanced, "{}{text}", ms_to_inline_tag(w_start)).ok()?;
        }
        rest = remaining;
    }

    if raw_enhanced.is_empty() {
        return None;
    }
    Some(format!("{line_tag}{raw_enhanced}"))
}

/// YRC token: `(startMs,durMs,0)text` (text until next `(` or end).
fn next_yrc_word(s: &str) -> Option<(i64, i64, &str, &str)> {
    if !s.starts_with('(') {
        return None;
    }
    let close = s.find(')')?;
    let inner = &s[1..close];
    let mut parts = inner.split(',');
    let w_start: i64 = parts.next()?.trim().parse().ok()?;
    let w_dur: i64 = parts.next()?.trim().parse().ok()?;
    let after = &s[close + 1..];
    let end = after.find('(').unwrap_or(after.len());
    let text = &after[..end];
    let remaining = &after[end..];
    Some((w_start, w_dur, text, remaining))
}

fn ms_to_line_tag(ms: i64) -> String {
    let cs = ms / 10;
    let mins = cs / 6000;
    let secs = (cs / 100) % 60;
    let frac = cs % 100;
    format!("[{mins:02}:{secs:02}.{frac:02}]")
}

fn ms_to_inline_tag(ms: i64) -> String {
    let cs = ms / 10;
    let mins = cs / 6000;
    let secs = (cs / 100) % 60;
    let frac = cs % 100;
    format!("<{mins:02}:{secs:02}.{frac:02}>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_json_metadata() {
        let yrc = r#"{"t":123}
12340,800(12500,200,0)爱(12700,300,0)你"#;
        let lrc = yrc_to_enhanced_lrc(yrc).expect("parsed");
        assert!(!lrc.contains('{'));
        assert!(lrc.contains("<00:12.50>爱"));
    }

    #[test]
    fn rejects_empty_yrc() {
        assert!(yrc_to_enhanced_lrc("{\"x\":1}\n").is_none());
    }
}