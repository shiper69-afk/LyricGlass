use parking_lot::Mutex;
use serde::Serialize;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperLastAttempt {
    pub timestamp_unix_ms: u64,
    pub trigger: String,
    pub outcome: String,
    pub attached_after: bool,
    pub error: Option<String>,
    pub trace: String,
}

static LAST_ATTEMPT: LazyLock<Mutex<Option<WallpaperLastAttempt>>> =
    LazyLock::new(|| Mutex::new(None));
static ATTEMPT_COUNT: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(0));

pub fn log_folder() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("LyricGlass")
}

pub fn log_file_path() -> PathBuf {
    log_folder().join("wallpaper.log")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn last_attempt() -> Option<WallpaperLastAttempt> {
    LAST_ATTEMPT.lock().clone()
}

pub fn attempt_count() -> u64 {
    *ATTEMPT_COUNT.lock()
}

pub fn record_attempt(
    trigger: &str,
    outcome: &str,
    attached_after: bool,
    error: Option<String>,
    trace: String,
) {
    let ts = now_ms();
  *ATTEMPT_COUNT.lock() += 1;
    let record = WallpaperLastAttempt {
        timestamp_unix_ms: ts,
        trigger: trigger.to_string(),
        outcome: outcome.to_string(),
        attached_after,
        error: error.clone(),
        trace: trace.clone(),
    };
    *LAST_ATTEMPT.lock() = Some(record);

    let header = format!(
        "\n=== {ts} trigger={trigger} outcome={outcome} attached={attached_after} error={} ===\n",
        error.as_deref().unwrap_or("—")
    );
    append_log_file(&(header + &trace));
}

pub fn append_log_file(body: &str) {
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        let _ = create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{body}");
        let _ = f.flush();
    }
}
