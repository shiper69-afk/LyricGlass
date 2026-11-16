use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static MAIN_HEARTBEAT_MS: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_STARTED: OnceLock<()> = OnceLock::new();
static WATCHDOG_ARMED: AtomicBool = AtomicBool::new(false);

static LOG_PATH: OnceLock<Mutex<PathBuf>> = OnceLock::new();

pub fn log_directory() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData").join("Local")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("LyricGlass").join("logs")
}

fn ensure_log_file() -> PathBuf {
    if let Some(mutex) = LOG_PATH.get() {
        return mutex.lock().unwrap().clone();
    }
    let dir = log_directory();
    let _ = create_dir_all(&dir);
    let name = format!(
        "lyricglass-{}.log",
        time::OffsetDateTime::now_utc().unix_timestamp()
    );
    let path = dir.join(name);
    let _ = LOG_PATH.set(Mutex::new(path.clone()));
    path
}

pub fn log_path_display() -> String {
    ensure_log_file().display().to_string()
}

pub fn log_diagnostic(block: &str) {
    let path = ensure_log_file();
    let ts = time::OffsetDateTime::now_utc().unix_timestamp();
    let body = format!("{ts} [diagnostics]\n{block}\n---\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(body.as_bytes());
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Updated only from the Tauri main event loop (`RunEvent`), not from background threads.
pub fn main_heartbeat() {
    MAIN_HEARTBEAT_MS.store(unix_ms(), Ordering::Relaxed);
}

/// Begin stall detection after the event loop is running (avoids false positives during setup).
pub fn arm_main_watchdog() {
    WATCHDOG_ARMED.store(true, Ordering::Relaxed);
    main_heartbeat();
}

pub fn start_main_heartbeat_pinger(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let app = app.clone();
        let _ = app.run_on_main_thread(|| main_heartbeat());
    });
}

fn start_main_watchdog() {
    WATCHDOG_STARTED.get_or_init(|| {
        std::thread::spawn(|| loop {
            std::thread::sleep(Duration::from_secs(2));
            if !WATCHDOG_ARMED.load(Ordering::Relaxed) {
                continue;
            }
            let last = MAIN_HEARTBEAT_MS.load(Ordering::Relaxed);
            let now = unix_ms();
            if now.saturating_sub(last) > 2000 {
                let path = ensure_log_file();
                let ts = time::OffsetDateTime::now_utc().unix_timestamp();
                let line = format!(
                    "{ts} [watchdog] main thread stall suspected (>2s since heartbeat; last={last} now={now})\n"
                );
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                {
                    let _ = file.write_all(line.as_bytes());
                }
            }
        });
    });
}

/// Background-thread logging (media watcher) — does not touch the main-loop heartbeat.
pub fn watcher_log(message: &str) {
    append_log_line("media", message);
}

fn append_log_line(kind: &str, message: &str) {
    let path = ensure_log_file();
    let ts = time::OffsetDateTime::now_utc().unix_timestamp();
    let line = format!("{ts} [{kind}] {message}\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
    #[cfg(debug_assertions)]
    eprintln!("{line}");
}

pub fn step(message: &str) {
    append_log_line("startup", message);
}

pub fn log_panic(payload: &str, location: &str, backtrace: &std::backtrace::Backtrace) {
    let path = ensure_log_file();
    let body = format!(
        "PANIC at {location}\n{payload}\n\nBacktrace:\n{backtrace}\n"
    );
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(body.as_bytes());
    }
}

#[cfg(windows)]
pub fn show_error_dialog(title: &str, message: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let wide: Vec<u16> = format!("{message}\n\nLog: {}", log_path_display())
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = MessageBoxW(
            None,
            windows::core::PCWSTR(wide.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
pub fn show_error_dialog(_title: &str, message: &str) {
    eprintln!("{message}");
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".into());
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".into());
        let backtrace = std::backtrace::Backtrace::force_capture();
        log_panic(&payload, &location, &backtrace);
        step(&format!("panic: {payload} @ {location}"));
        show_error_dialog(
            "LyricGlass crashed",
            &format!("LyricGlass encountered a fatal error.\n\n{payload}\n\nSee log for details."),
        );
    }));
}

pub fn log_fatal_error(context: &str, err: impl std::fmt::Display) {
    step(&format!("fatal ({context}): {err}"));
    show_error_dialog(
        "LyricGlass could not start",
        &format!("{context}: {err}"),
    );
}

pub fn init_logging() {
    step("logging initialized");
    step(&format!("log file: {}", log_path_display()));
    start_main_watchdog();
}