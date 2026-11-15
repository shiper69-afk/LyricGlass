/// Playback clock anchored to GSMTC `Position` at `LastUpdatedTime`.
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
    anchor_unix_ms: u64,
    playing: bool,
    playback_rate: f64,
    now_unix_ms: u64,
) -> u64 {
    let elapsed = now_unix_ms.saturating_sub(anchor_unix_ms);
    smoothed_position_ms(position_ms, playing, elapsed, playback_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_holds_position() {
        assert_eq!(smoothed_position_ms(90_000, false, 5_000, 1.0), 90_000);
    }

    #[test]
    fn playing_advances() {
        assert_eq!(smoothed_position_ms(90_000, true, 1_500, 1.0), 91_500);
    }

    #[test]
    fn stale_gsmtc_extrapolation() {
        assert_eq!(smoothed_position_ms(30_000, true, 3_000, 1.0), 33_000);
    }

    #[test]
    fn playback_rate_applied() {
        assert_eq!(smoothed_position_ms(10_000, true, 2_000, 1.25), 12_500);
    }

    #[test]
    fn seek_via_media_clock() {
        let now = 1_000_100;
        let clock = media_clock_ms(120_000, 1_000_050, true, 1.0, now);
        assert_eq!(clock, 120_050);
    }

    #[test]
    fn pause_stops_advance() {
        let now = 1_005_000;
        let clock = media_clock_ms(60_000, 1_000_000, false, 1.0, now);
        assert_eq!(clock, 60_000);
    }

    #[test]
    fn track_change_fresh_anchor() {
        let now = 5_000_250;
        let clock = media_clock_ms(0, 5_000_000, true, 1.0, now);
        assert_eq!(clock, 250);
    }

    #[test]
    fn saturating_at_max() {
        assert_eq!(
            smoothed_position_ms(u64::MAX - 10, true, 100, 1.0),
            u64::MAX
        );
    }
}