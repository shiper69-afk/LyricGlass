use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

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

pub fn step(message: &str) {
    let path = ensure_log_file();
    let ts = time::OffsetDateTime::now_utc().unix_timestamp();
    let line = format!("{ts} [startup] {message}\n");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
    #[cfg(debug_assertions)]
    eprintln!("{line}");
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
}