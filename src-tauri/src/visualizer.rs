//! WASAPI loopback capture, FFT, and throttled `visualizer-frame` events.

mod dsp {
    pub use crate::visualizer_dsp::*;
}

use crate::lyrics::SharedLyricsService;
use crate::media::SharedMediaState;
use crate::startup;
use dsp::{
    bands_to_u8, detect_beat, magnitudes_to_bands, overall_level, smooth_bands, BeatState,
    BAND_COUNT, FFT_SIZE,
};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub static UI_ACTIVE: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Serialize)]
struct VisualizerFramePayload {
    #[serde(rename = "b")]
    bands: Vec<u8>,
    #[serde(rename = "l")]
    level: u8,
    #[serde(rename = "bt")]
    beat: bool,
}

#[derive(Clone)]
struct RuntimeConfig {
    kind: String,
    intensity: f64,
}

fn read_config(service: &SharedLyricsService) -> RuntimeConfig {
    RuntimeConfig {
        kind: service.get_visualizer_kind(),
        intensity: service.get_visualizer_intensity(),
    }
}

fn media_allows_capture(media: &crate::media::MediaState) -> bool {
    let st = media.playback_status.to_ascii_lowercase();
    st == "playing"
}

fn overlay_window_visible(app: &AppHandle) -> bool {
    use crate::wallpaper;
    if wallpaper::is_attached() {
        app.get_webview_window("wallpaper")
            .map(|w| w.is_visible().unwrap_or(false))
            .unwrap_or(false)
    } else {
        app.get_webview_window("main")
            .map(|w| w.is_visible().unwrap_or(true))
            .unwrap_or(true)
    }
}

fn should_run(app: &AppHandle, service: &SharedLyricsService, media: &crate::media::MediaState) -> bool {
    let cfg = read_config(service);
    if cfg.kind == "off" {
        return false;
    }
    if !UI_ACTIVE.load(Ordering::Relaxed) {
        return false;
    }
    if !overlay_window_visible(app) {
        return false;
    }
    media_allows_capture(media)
}

pub fn start(app: AppHandle, media: SharedMediaState, service: SharedLyricsService) {
    let poll_app = app.clone();
    std::thread::spawn(move || visibility_poll_loop(poll_app));
    std::thread::spawn(move || {
        startup::step("visualizer: coordinator thread started");
        #[cfg(windows)]
        audio_loop(app, media, service);
        #[cfg(not(windows))]
        {
            let _ = (app, media, service);
            startup::step("visualizer: disabled on non-Windows");
        }
    });
}

fn visibility_poll_loop(app: AppHandle) {
    loop {
        let vis = overlay_window_visible(&app);
        UI_ACTIVE.store(vis, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[cfg(windows)]
fn audio_loop(app: AppHandle, media: SharedMediaState, service: SharedLyricsService) {
    use realfft::num_complex::Complex;
    use realfft::RealFftPlanner;
    use std::collections::VecDeque;

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(FFT_SIZE);
    let mut scratch = r2c.make_scratch_vec();
    let mut indata = vec![0.0f32; FFT_SIZE];
    let mut spectrum = r2c.make_output_vec();

    let mut smoothed = vec![0.0f32; BAND_COUNT];
    let mut beat_state = BeatState::default();
    let mut sample_buf: VecDeque<f32> = VecDeque::with_capacity(FFT_SIZE * 2);
    let mut last_loud = Instant::now();
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    const EMIT_INTERVAL: Duration = Duration::from_millis(33);
    const SILENCE_HOLD: Duration = Duration::from_secs(2);

    loop {
        let snap = media.read().clone();
        if !should_run(&app, &service, &snap) {
            std::thread::sleep(Duration::from_millis(200));
            sample_buf.clear();
            continue;
        }

        match read_loopback_chunk() {
            Ok(chunk) => {
                for s in chunk {
                    sample_buf.push_back(s);
                }
            }
            Err(e) => {
                startup::step(&format!("visualizer: loopback paused ({e})"));
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        }

        while sample_buf.len() >= FFT_SIZE {
            for i in 0..FFT_SIZE {
                indata[i] = sample_buf.pop_front().unwrap_or(0.0);
            }
            r2c.process_with_scratch(&mut indata, &mut spectrum, &mut scratch)
                .ok();
            let mags: Vec<f32> = spectrum
                .iter()
                .map(|c: &Complex<f32>| c.norm())
                .collect();
            let raw = magnitudes_to_bands(&mags, BAND_COUNT);
            smoothed = smooth_bands(&smoothed, &raw, 0.45, 0.12);
            let level = overall_level(&smoothed);
            if level > 0.04 {
                last_loud = Instant::now();
            }
            if last_loud.elapsed() > SILENCE_HOLD {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }

            let low: f32 = smoothed.iter().take(6).sum::<f32>() / 6.0;
            let beat = detect_beat(low, &mut beat_state);
            let cfg = read_config(&service);
            let bands_u8 = bands_to_u8(&smoothed, cfg.intensity as f32);

            if last_emit.elapsed() >= EMIT_INTERVAL {
                last_emit = Instant::now();
                let payload = VisualizerFramePayload {
                    bands: bands_u8,
                    level: (level * 255.0).round() as u8,
                    beat,
                };
                let _ = app.emit("visualizer-frame", payload);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(windows)]
fn read_loopback_chunk() -> Result<Vec<f32>, String> {
    use std::cell::RefCell;
    use wasapi::{
        get_default_device, initialize_mta, Direction, SampleType, StreamMode, WaveFormat,
    };

    thread_local! {
        static CAPTURE: RefCell<Option<LoopbackState>> = const { RefCell::new(None) };
    }

    struct LoopbackState {
        client: wasapi::AudioClient,
        capture: wasapi::AudioCaptureClient,
        block_align: usize,
        channels: u16,
    }

    CAPTURE.with(|cell| {
        if cell.borrow().is_none() {
            initialize_mta()
                .ok()
                .map_err(|e| format!("initialize_mta: {e}"))?;
            let device = get_default_device(&Direction::Render).map_err(|e| e.to_string())?;
            let mut audio_client = device.get_iaudioclient().map_err(|e| e.to_string())?;
            let format = WaveFormat::new(32, 32, &SampleType::Float, 48_000, 2, None);
            let (_, min_time) = audio_client.get_device_period().map_err(|e| e.to_string())?;
            let mode = StreamMode::PollingShared {
                autoconvert: true,
                buffer_duration_hns: min_time,
            };
            audio_client
                .initialize_client(&format, &Direction::Capture, &mode)
                .map_err(|e| e.to_string())?;
            audio_client.start_stream().map_err(|e| e.to_string())?;
            let capture = audio_client.get_audiocaptureclient().map_err(|e| e.to_string())?;
            *cell.borrow_mut() = Some(LoopbackState {
                block_align: format.get_blockalign() as usize,
                channels: format.get_nchannels(),
                client: audio_client,
                capture,
            });
            startup::step("visualizer: WASAPI loopback ready");
        }

        let mut state = cell.borrow_mut();
        let st = state.as_mut().ok_or("visualizer not init")?;
        let mut deque: VecDeque<u8> = VecDeque::new();
        st.capture
            .read_from_device_to_deque(&mut deque)
            .map_err(|e| e.to_string())?;
        if deque.is_empty() {
            return Ok(vec![]);
        }
        let bytes: Vec<u8> = deque.into();
        let mut mono = Vec::with_capacity(bytes.len() / (st.block_align.max(1)));
        let stride = st.block_align;
        for chunk in bytes.chunks(stride) {
            if chunk.len() < 4 {
                continue;
            }
            let l = f32::from_le_bytes(chunk[0..4].try_into().unwrap_or([0; 4]));
            let r = if chunk.len() >= 8 {
                f32::from_le_bytes(chunk[4..8].try_into().unwrap_or([0; 4]))
            } else {
                l
            };
            mono.push((l + r) * 0.5);
        }
        Ok(mono)
    })
}

#[tauri::command]
pub fn set_visualizer_ui_active(active: bool) {
    UI_ACTIVE.store(active, Ordering::Relaxed);
}