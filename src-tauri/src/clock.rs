const WINDOWS_TICK_OFFSET: i64 = 11_644_473_600_000_000;

/// WinRT `DateTime.UniversalTime` (100 ns since 1601-01-01 UTC) → Unix ms.
pub fn winrt_date_time_to_unix_ms(universal_time_100ns: i64) -> Option<u64> {
    if universal_time_100ns < WINDOWS_TICK_OFFSET {
        return None;
    }
    let unix_100ns = universal_time_100ns.saturating_sub(WINDOWS_TICK_OFFSET);
    Some((unix_100ns / 10_000) as u64)
}

pub fn smoothed_position_ms(
    position_ms: u64,
    playing: bool,
    elapsed_since_anchor_ms: u64,
    playback_rate: f64,
) -> u64 {
    if !playing || playback_rate <= 0.0 {
        return position_ms;
    }
    let extra = (elapsed_since_anchor_ms as f64) * playback_rate;
    position_ms.saturating_add(extra.floor() as u64)
}

pub fn media_clock_ms(
    position_ms: u64,
    gsmtc_anchor_ms: u64,
    wall_anchor_ms: u64,
    playing: bool,
    playback_rate: f64,
    now_unix_ms: u64,
) -> u64 {
    if !playing {
        return position_ms;
    }
    let elapsed_gsmtc = safe_elapsed(now_unix_ms, gsmtc_anchor_ms);
    let elapsed_wall = safe_elapsed(now_unix_ms, wall_anchor_ms);
    let from_gsmtc = smoothed_position_ms(position_ms, true, elapsed_gsmtc, playback_rate);
    let from_wall = smoothed_position_ms(position_ms, true, elapsed_wall, playback_rate);
    from_gsmtc.max(from_wall)
}

fn safe_elapsed(now_ms: u64, anchor_ms: u64) -> u64 {
    if anchor_ms == 0 {
        return 0;
    }
    now_ms.saturating_sub(anchor_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winrt_epoch_unix_zero() {
        let unix = winrt_date_time_to_unix_ms(WINDOWS_TICK_OFFSET).unwrap();
        assert_eq!(unix, 0);
        let plus_one_sec = winrt_date_time_to_unix_ms(WINDOWS_TICK_OFFSET + 10_000_000).unwrap();
        assert_eq!(plus_one_sec, 1000);
    }

    #[test]
    fn paused_holds_position() {
        assert_eq!(smoothed_position_ms(90_000, false, 5_000, 1.0), 90_000);
    }

    #[test]
    fn stale_gsmtc_extrapolation() {
        assert_eq!(smoothed_position_ms(30_000, true, 3_000, 1.0), 33_000);
    }

    #[test]
    fn dual_anchor_uses_max_for_spotify_quirk() {
        let now = 1_000_000;
        // GSMTC anchor fresh (LastUpdatedTime = now): no extrapolation
        // Wall anchor 4s ago: extrapolate +4s
        let clock = media_clock_ms(30_000, now, now - 4_000, true, 1.0, now);
        assert_eq!(clock, 34_000);
    }

    #[test]
    fn future_anchor_clamped() {
        let now = 5_000;
        let clock = media_clock_ms(1_000, 9_000, 9_000, true, 1.0, now);
        assert_eq!(clock, 1_000);
    }

    #[test]
    fn seek_via_media_clock() {
        let now = 1_000_100;
        let clock = media_clock_ms(120_000, 1_000_050, 1_000_050, true, 1.0, now);
        assert_eq!(clock, 120_050);
    }
}