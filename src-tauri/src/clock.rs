/// Shared interpolation logic for unit tests and documentation.
pub fn smoothed_position_ms(
    last_reported_ms: u64,
    playing: bool,
    elapsed_since_report_ms: u64,
) -> u64 {
    if !playing {
        return last_reported_ms;
    }
    last_reported_ms.saturating_add(elapsed_since_report_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_holds_position() {
        assert_eq!(smoothed_position_ms(90_000, false, 5_000), 90_000);
    }

    #[test]
    fn playing_advances() {
        assert_eq!(smoothed_position_ms(90_000, true, 1_500), 91_500);
    }

    #[test]
    fn saturating_at_max() {
        assert_eq!(smoothed_position_ms(u64::MAX - 10, true, 100), u64::MAX);
    }
}