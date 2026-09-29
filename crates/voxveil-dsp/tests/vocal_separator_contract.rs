mod common;

use common::*;
use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::{SeparatorEvalTap, VocalSeparator, separator_fft_size};
use voxveil_types::VocalLevel;

fn level(value: f32) -> VocalLevel {
    VocalLevel::new(value).unwrap()
}

fn max_delayed_error(output: &[f32], reference: &[f32], latency: usize, skip: usize) -> f32 {
    let frames = reference.len() / 2;
    (skip..frames)
        .flat_map(|n| {
            let o = 2 * (n + latency);
            [(output[o] - reference[2 * n]).abs(), (output[o + 1] - reference[2 * n + 1]).abs()]
        })
        .fold(0.0, f32::max)
}

#[test]
fn latency_follows_sample_rate() {
    for (rate, expected) in [(44_100, 4_095), (48_000, 4_095), (96_000, 8_191)] {
        let separator = VocalSeparator::new(rate, level(0.0));
        assert_eq!(separator.latency_frames(), expected, "rate={rate}");
        assert_eq!(separator_fft_size(rate), expected + 1);
    }
}

#[test]
fn full_vocal_level_is_transparent_after_latency() {
    let mut separator = VocalSeparator::new(RATE, level(1.0));
    let latency = separator.latency_frames();
    let reference = add(&centered(&voice(24_000)), &side(&sine(24_000, 700.0, 0.2)));
    let mut buffer = reference.clone();
    buffer.extend(std::iter::repeat_n(0.0, latency * 2));
    separator.process_stereo_interleaved(&mut buffer);
    let error = max_delayed_error(&buffer, &reference, latency, 4_096);
    assert!(error <= 0.002, "error={error}");
}

#[test]
fn hot_switch_to_full_vocal_level_recovers_transparency() {
    let mut separator = VocalSeparator::new(RATE, level(0.0));
    let mut warm = centered(&voice(24_000));
    separator.process_stereo_interleaved(&mut warm);
    separator.set_vocal_level(level(1.0));
    let latency = separator.latency_frames();
    let reference = centered(&sine(24_000, 1_000.0, 0.4));
    let mut buffer = reference.clone();
    buffer.extend(std::iter::repeat_n(0.0, latency * 2));
    separator.process_stereo_interleaved(&mut buffer);
    let error = max_delayed_error(&buffer, &reference, latency, 4_096);
    assert!(error <= 0.002, "error={error}");
}

#[test]
fn non_finite_and_odd_input_is_safe() {
    let mut separator = VocalSeparator::new(RATE, level(0.0));
    let mut buffer = centered(&voice(20_000));
    buffer[1_000] = f32::NAN;
    buffer[5_001] = f32::INFINITY;
    buffer.push(0.25); // odd trailing sample
    separator.process_stereo_interleaved(&mut buffer);
    assert!(buffer.iter().all(|v| v.is_finite()));
    assert_eq!(*buffer.last().unwrap(), 0.25);
}

#[test]
fn silence_stays_silent() {
    let mut separator = VocalSeparator::new(RATE, level(0.0));
    let mut buffer = vec![0.0; 40_000];
    separator.process_stereo_interleaved(&mut buffer);
    assert!(buffer.iter().all(|&v| v == 0.0));
}

#[test]
fn eval_tap_is_linear_in_the_stems() {
    let mut separator = VocalSeparator::new(RATE, level(0.0));
    let mut tap = SeparatorEvalTap::new(&separator);
    let vocal = centered(&voice(30_000));
    let instrumental = add(&centered(&drums(30_000)), &side(&sine(30_000, 900.0, 0.1)));
    let mut worst = 0.0_f32;
    for n in 0..30_000 {
        let v = [vocal[2 * n], vocal[2 * n + 1]];
        let i = [instrumental[2 * n], instrumental[2 * n + 1]];
        let [mix, v_out, i_out] = tap.process_frame(&mut separator, v, i);
        worst = worst
            .max((mix[0] - v_out[0] - i_out[0]).abs())
            .max((mix[1] - v_out[1] - i_out[1]).abs());
    }
    assert!(worst < 1.0e-3, "worst={worst}");
}
