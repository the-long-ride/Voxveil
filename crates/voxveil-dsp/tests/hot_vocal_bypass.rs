use std::f32::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

const SAMPLE_RATE: u32 = 48_000;
const WARM_FRAMES: usize = 4_096;
const TEST_FRAMES: usize = 8_192;
const SETTLE_FRAMES: usize = 1_024;

fn centered_tone(start_frame: usize, frames: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(frames * 2);
    for frame in start_frame..start_frame + frames {
        let value = (TAU * 1_000.0 * frame as f32 / SAMPLE_RATE as f32).sin() * 0.4;
        samples.push(value);
        samples.push(value);
    }
    samples
}

#[test]
fn hot_switch_to_full_vocal_recovers_transparency_after_fixed_latency() {
    let mut processor = SpectralCenterSuppressor::new(
        SAMPLE_RATE,
        VocalLevel::new(0.0).unwrap(),
        ClassicSuppressionProfile::Balanced,
    );

    let mut warm = centered_tone(0, WARM_FRAMES);
    processor.process_stereo_interleaved(&mut warm);

    processor.set_vocal_level(VocalLevel::new(1.0).unwrap());
    let latency = processor.latency_frames();
    let reference = centered_tone(WARM_FRAMES, TEST_FRAMES);
    let mut rendered = reference.clone();
    rendered.extend(std::iter::repeat_n(
        0.0,
        (latency + SETTLE_FRAMES) * 2,
    ));
    processor.process_stereo_interleaved(&mut rendered);

    let output_start = (latency + SETTLE_FRAMES) * 2;
    let output_end = (latency + TEST_FRAMES - SETTLE_FRAMES) * 2;
    let reference_start = SETTLE_FRAMES * 2;
    let reference_end = (TEST_FRAMES - SETTLE_FRAMES) * 2;
    let max_error = rendered[output_start..output_end]
        .iter()
        .zip(&reference[reference_start..reference_end])
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0_f32, f32::max);

    assert!(max_error <= 0.002, "max_error={max_error}");
}
