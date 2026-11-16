// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == "--wallpaper-selftest") {
        std::process::exit(lyricglass_lib::run_wallpaper_selftest());
    }
    lyricglass_lib::run()
}