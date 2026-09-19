use std::f32::consts::TAU;

use super::*;

const SAMPLE_RATE: u32 = 48_000;
const FRAMES: usize = 8_192;
const SETTLE_FRAMES: usize = 1_024;

fn tone(freq_hz: f32, side_only: bool) -> Vec<f32> {
    let mut samples = Vec::with_capacity(FRAMES * 2);
    for frame in 0..FRAMES {
        let value = (TAU * freq_hz * frame as f32 / SAMPLE_RATE as f32).sin() * 0.4;
        samples.push(value);
        samples.push(if side_only { -value } else { value });
    }
    samples
}

fn rms(samples: &[f32]) -> f32 {
    let sum = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / samples.len().max(1) as f32).sqrt()
}

fn render(
    input: &[f32],
    vocal_level: f32,
    profile: ClassicSuppressionProfile,
) -> (Vec<f32>, usize) {
    let mut processor = SpectralCenterSuppressor::new(
        SAMPLE_RATE,
        VocalLevel::new(vocal_level).unwrap(),
        profile,
    );
    let latency = processor.latency_frames();
    let mut stream = input.to_vec();
    stream.extend(std::iter::repeat_n(0.0, (latency + 1_024) * 2));
    for chunk in stream.chunks_mut(512) {
        processor.process_stereo_interleaved(chunk);
    }
    (stream, latency)
}

fn settled_left_channel(stream: &[f32], latency: usize) -> Vec<f32> {
    let start = (latency + SETTLE_FRAMES) * 2;
    let end = (latency + FRAMES - SETTLE_FRAMES) * 2;
    stream[start..end].chunks_exact(2).map(|frame| frame[0]).collect()
}

#[test]
fn balanced_reduces_sustained_center_vocal_band_by_at_least_eight_db() {
    let input = tone(1_000.0, false);
    let input_rms = rms(&input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]);
    let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
    let output_rms = rms(&settled_left_channel(&output, latency));
    assert!(output_rms / input_rms <= 0.40, "ratio={}", output_rms / input_rms);
}

#[test]
fn music_preservation_keeps_centered_low_bass_within_three_db() {
    let input = tone(90.0, false);
    let input_rms = rms(&input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]);
    let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::MusicPreservation);
    let output_rms = rms(&settled_left_channel(&output, latency));
    assert!(output_rms / input_rms >= 0.70, "ratio={}", output_rms / input_rms);
}

#[test]
fn balanced_is_stronger_than_music_preservation_in_vocal_band() {
    let input = tone(1_000.0, false);
    let (music, music_latency) = render(&input, 0.0, ClassicSuppressionProfile::MusicPreservation);
    let (balanced, balanced_latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
    let music_rms = rms(&settled_left_channel(&music, music_latency));
    let balanced_rms = rms(&settled_left_channel(&balanced, balanced_latency));
    assert!(balanced_rms < music_rms * 0.8, "balanced={balanced_rms}, music={music_rms}");
}

#[test]
fn side_only_signal_is_preserved() {
    let input = tone(1_000.0, true);
    let input_rms = rms(&input[SETTLE_FRAMES * 2..(FRAMES - SETTLE_FRAMES) * 2]);
    let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
    let output_rms = rms(&settled_left_channel(&output, latency));
    let ratio = output_rms / input_rms;
    assert!((0.944..=1.059).contains(&ratio), "ratio={ratio}");
}

#[test]
fn full_vocal_level_is_transparent_after_latency() {
    let input = tone(777.0, false);
    let input_left: Vec<f32> = input.chunks_exact(2).map(|frame| frame[0]).collect();
    let (output, latency) = render(&input, 1.0, ClassicSuppressionProfile::Balanced);
    let output_left = settled_left_channel(&output, latency);
    let reference = &input_left[SETTLE_FRAMES..FRAMES - SETTLE_FRAMES];
    let max_error = output_left
        .iter()
        .zip(reference)
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0_f32, f32::max);
    assert!(max_error <= 0.002, "max_error={max_error}");
}

#[test]
fn mono_signal_remains_audible_at_maximum_suppression() {
    let input = tone(1_000.0, false);
    for profile in [
        ClassicSuppressionProfile::MusicPreservation,
        ClassicSuppressionProfile::Balanced,
    ] {
        let (output, latency) = render(&input, 0.0, profile);
        assert!(rms(&settled_left_channel(&output, latency)) > 0.03);
    }
}

#[test]
fn output_is_finite_and_latency_stays_below_twenty_five_ms() {
    let input = tone(1_000.0, false);
    let (output, latency) = render(&input, 0.0, ClassicSuppressionProfile::Balanced);
    assert!(output.iter().all(|sample| sample.is_finite()));
    assert!(latency <= (SAMPLE_RATE as usize * 25 / 1_000));
}
