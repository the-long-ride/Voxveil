use std::f32::consts::TAU;
use std::time::Instant;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::VocalSeparator;
use voxveil_types::VocalLevel;

fn main() {
    let rate = 48_000_u32;
    let seconds = 60;
    let frames = rate as usize * seconds;
    let mut buffer: Vec<f32> = (0..frames)
        .flat_map(|n| {
            let t = n as f32 / rate as f32;
            let center = (TAU * 220.0 * t).sin() * 0.2 + (TAU * 55.0 * t).sin() * 0.2;
            let side = (TAU * 1_234.0 * t).sin() * 0.1;
            [center + side, center - side]
        })
        .collect();
    let mut separator = VocalSeparator::new(rate, VocalLevel::new(0.0).unwrap());
    let start = Instant::now();
    for chunk in buffer.chunks_mut(960) {
        separator.process_stereo_interleaved(chunk);
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "{seconds} s of 48 kHz stereo in {elapsed:.3} s -> {:.2}% of one core",
        100.0 * elapsed / seconds as f64
    );
}
