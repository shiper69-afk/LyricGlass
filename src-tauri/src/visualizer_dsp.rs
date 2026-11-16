//! FFT band mapping, smoothing, and beat detection (unit-testable).

pub const BAND_COUNT: usize = 48;
pub const FFT_SIZE: usize = 2048;

/// Map linear FFT magnitude bins to `BAND_COUNT` log-spaced bands (0..1).
pub fn magnitudes_to_bands(mags: &[f32], band_count: usize) -> Vec<f32> {
    if mags.is_empty() || band_count == 0 {
        return vec![0.0; band_count];
    }
    let mut out = vec![0.0f32; band_count];
    let max_bin = mags.len().saturating_sub(1);
    for b in 0..band_count {
        let t0 = b as f32 / band_count as f32;
        let t1 = (b + 1) as f32 / band_count as f32;
        let i0 = ((t0 * t0 * max_bin as f32).floor() as usize).min(max_bin);
        let i1 = ((t1 * t1 * max_bin as f32).ceil() as usize).min(max_bin);
        if i1 <= i0 {
            out[b] = mags[i0.min(max_bin)];
        } else {
            let slice = &mags[i0..=i1];
            out[b] = slice.iter().sum::<f32>() / slice.len() as f32;
        }
    }
    normalize_bands(&mut out);
    out
}

pub fn normalize_bands(bands: &mut [f32]) {
    let peak = bands.iter().copied().fold(0.0f32, f32::max);
    let scale = if peak > 1e-6 { 1.0 / peak } else { 1.0 };
    for v in bands.iter_mut() {
        *v = (*v * scale).clamp(0.0, 1.0);
    }
}

pub fn smooth_bands(prev: &[f32], next: &[f32], attack: f32, release: f32) -> Vec<f32> {
    let n = prev.len().min(next.len());
    let mut out = vec![0.0; n];
    for i in 0..n {
        let p = prev.get(i).copied().unwrap_or(0.0);
        let t = next[i];
        let alpha = if t > p { attack } else { release };
        out[i] = p + (t - p) * alpha;
    }
    out
}

pub fn overall_level(bands: &[f32]) -> f32 {
    if bands.is_empty() {
        return 0.0;
    }
    bands.iter().sum::<f32>() / bands.len() as f32
}

/// Simple energy-flux beat detector on low bands.
pub fn detect_beat(low_energy: f32, state: &mut BeatState) -> bool {
    const THRESH: f32 = 0.12;
    const COOLDOWN: u32 = 8;
    let flux = low_energy - state.prev_low;
    state.prev_low = low_energy;
    if state.cooldown > 0 {
        state.cooldown -= 1;
        return false;
    }
    if flux > THRESH && low_energy > 0.15 {
        state.cooldown = COOLDOWN;
        return true;
    }
    false
}

#[derive(Clone, Debug, Default)]
pub struct BeatState {
    pub prev_low: f32,
    pub cooldown: u32,
}

pub fn bands_to_u8(bands: &[f32], intensity: f32) -> Vec<u8> {
    bands
        .iter()
        .map(|v| (v * intensity.clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magnitudes_to_bands_len_and_range() {
        let mags: Vec<f32> = (0..512).map(|i| i as f32).collect();
        let bands = magnitudes_to_bands(&mags, BAND_COUNT);
        assert_eq!(bands.len(), BAND_COUNT);
        assert!(bands[0] <= bands[BAND_COUNT - 1] + 0.01);
        assert!(bands.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }

    #[test]
    fn smooth_attack_faster_than_release() {
        let prev = vec![0.0; 4];
        let next = vec![1.0; 4];
        let up = smooth_bands(&prev, &next, 0.5, 0.1);
        let down = smooth_bands(&next, &prev, 0.5, 0.1);
        let rise = up[0] - prev[0];
        let fall = next[0] - down[0];
        assert!(rise > fall, "attack should move bands up faster than release pulls down");
    }

    #[test]
    fn beat_fires_on_energy_jump() {
        let mut st = BeatState::default();
        assert!(!detect_beat(0.05, &mut st));
        assert!(detect_beat(0.35, &mut st));
        assert!(!detect_beat(0.36, &mut st));
    }
}