//! Counters for CI / unit asserts (brief: 0x52C at most once per attach attempt).

use std::sync::atomic::{AtomicU32, Ordering};

static PROGMAN_052C_SENDS: AtomicU32 = AtomicU32::new(0);

pub fn should_send_052c_message(workerw_already_present: bool) -> bool {
    !workerw_already_present
}

pub fn record_progman_052c_send() {
    PROGMAN_052C_SENDS.fetch_add(1, Ordering::Relaxed);
}

pub fn progman_052c_send_count() -> u32 {
    PROGMAN_052C_SENDS.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_052c_when_workerw_present() {
        assert!(!should_send_052c_message(true));
        assert!(should_send_052c_message(false));
    }
}