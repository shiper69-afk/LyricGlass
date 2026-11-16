//! Semantic version parsing and comparison for upgrade handoff.

use std::cmp::Ordering;

/// Normalize PE `FileVersion` strings (`0, 7, 2, 0` → `0.7.2.0`).
pub fn normalize_version_string(s: &str) -> String {
    s.trim()
        .replace(',', ".")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Parse `major.minor.patch` (optional `v` prefix; patch may include non-digit suffix).
pub fn parse_version_triplet(s: &str) -> Option<(u32, u32, u32)> {
    let s = normalize_version_string(s);
    let s = s.trim_start_matches(['v', 'V']);
    let core = s.split_whitespace().next()?;
    let mut parts = core.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch_part = parts.next().unwrap_or("0");
    let patch_digits: String = patch_part
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let patch: u32 = if patch_digits.is_empty() {
        0
    } else {
        patch_digits.parse().ok()?
    };
    Some((major, minor, patch))
}

pub fn compare_version_strings(a: &str, b: &str) -> Ordering {
    let a = parse_version_triplet(a).unwrap_or((0, 0, 0));
    let b = parse_version_triplet(b).unwrap_or((0, 0, 0));
    a.cmp(&b)
}

/// `true` when `incoming` is strictly newer than `running`.
pub fn incoming_is_newer(incoming: &str, running: &str) -> bool {
    compare_version_strings(incoming, running) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        assert_eq!(
            parse_version_triplet("0.7.2"),
            Some((0, 7, 2))
        );
        assert_eq!(
            parse_version_triplet("v1.0.0"),
            Some((1, 0, 0))
        );
        assert_eq!(
            parse_version_triplet("0.7.1-beta"),
            Some((0, 7, 1))
        );
    }

    #[test]
    fn compare_patch_minor_major() {
        assert_eq!(compare_version_strings("0.7.2", "0.7.1"), Ordering::Greater);
        assert_eq!(compare_version_strings("0.7.1", "0.7.1"), Ordering::Equal);
        assert_eq!(compare_version_strings("0.6.9", "0.7.0"), Ordering::Less);
        assert_eq!(compare_version_strings("1.0.0", "0.9.9"), Ordering::Greater);
    }

    #[test]
    fn incoming_is_newer_cases() {
        assert!(incoming_is_newer("0.7.2", "0.7.0"));
        assert!(!incoming_is_newer("0.7.0", "0.7.2"));
        assert!(!incoming_is_newer("0.7.1", "0.7.1"));
    }

    #[test]
    fn normalize_pe_style() {
        assert_eq!(normalize_version_string("0, 7, 2, 0"), "0.7.2.0");
        assert_eq!(
            parse_version_triplet("0, 7, 2, 0"),
            Some((0, 7, 2))
        );
    }
}