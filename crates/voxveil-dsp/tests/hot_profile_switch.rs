use std::f32::consts::TAU;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

const SAMPLE_RATE: u32 = 48_000;
const WARM_FRAMES: usize = 8_192;
const POST_FRAMES: usize = 4_096;
const MEASURE_FRAMES: usize = 512;

fn centered_tone(start_frame: usize, frames: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(frames * 2);
    for frame in start_frame..start_frame + frames {
        let value = (TAU * 1_000.0 * frame as f32 / SAMPLE_RATE as f32).sin() * 0.4;
        samples.push(value);
        samples.push(value);
    }
    samples
}

fn left_rms(samples: &[f32], start_frame: usize, frames: usize) -> f32 {
    let start = start_frame * 2;
    let end = (start_frame + frames) * 2;
    let mut sum = 0.0_f32;
    let mut count = 0_usize;
    for frame in samples[start..end].chunks_exact(2) {
        sum += frame[0] * frame[0];
        count += 1;
    }
    (sum / count.max(1) as f32).sqrt()
}

#[test]
fn balanced_to_music_hot_switch_does_not_inherit_stronger_balanced_attenuation() {
    let mut switched = SpectralCenterSuppressor::new(
        SAMPLE_RATE,
        VocalLevel::new(0.0).unwrap(),
        ClassicSuppressionProfile::Balanced,
    );
    let mut music_reference = SpectralCenterSuppressor::new(
        SAMPLE_RATE,
        VocalLevel::new(0.0).unwrap(),
        ClassicSuppressionProfile::MusicPreservation,
    );

    let warm_input = centered_tone(0, WARM_FRAMES);
    let mut balanced_warm = warm_input.clone();
    let mut music_warm = warm_input;
    switched.process_stereo_interleaved(&mut balanced_warm);
    music_reference.process_stereo_interleaved(&mut music_warm);

    let warm_start = WARM_FRAMES - MEASURE_FRAMES;
    let balanced_rms = left_rms(&balanced_warm, warm_start, MEASURE_FRAMES);
    let music_rms = left_rms(&music_warm, warm_start, MEASURE_FRAMES);
    assert!(
        balanced_rms < music_rms * 0.8,
        "precondition failed: balanced={balanced_rms}, music={music_rms}"
    );

    switched.set_profile(ClassicSuppressionProfile::MusicPreservation);
    let latency = switched.latency_frames();
    assert_eq!(latency, music_reference.latency_frames());

    let post_input = centered_tone(WARM_FRAMES, POST_FRAMES + latency);
    let mut switched_output = post_input.clone();
    let mut reference_output = post_input;
    switched.process_stereo_interleaved(&mut switched_output);
    music_reference.process_stereo_interleaved(&mut reference_output);

    let switched_rms = left_rms(&switched_output, latency, MEASURE_FRAMES);
    let reference_rms = left_rms(&reference_output, latency, MEASURE_FRAMES);
    assert!(
        switched_rms >= reference_rms * 0.95,
        "Music preservation inherited stronger Balanced attenuation: switched={switched_rms}, reference={reference_rms}"
    );
}
