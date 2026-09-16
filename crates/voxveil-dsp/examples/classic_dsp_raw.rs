use std::env;
use std::io::{self, Read, Write};

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

const FRAME_BYTES: usize = 8;
const READ_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy)]
struct Config {
    sample_rate: u32,
    vocal_level: VocalLevel,
    profile: ClassicSuppressionProfile,
}

fn usage() -> &'static str {
    "usage: classic_dsp_raw [--sample-rate HZ] [--vocal 0..100] [--profile music-preservation|balanced]\n\
reads stereo f32le from stdin and writes latency-compensated stereo f32le to stdout"
}

fn parse_args() -> Result<Config, String> {
    let mut sample_rate = 48_000_u32;
    let mut vocal_percent = 0_u8;
    let mut profile = ClassicSuppressionProfile::MusicPreservation;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sample-rate" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--sample-rate requires a value".to_string())?;
                sample_rate = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid sample rate: {value}"))?;
                if sample_rate < 8_000 {
                    return Err("sample rate must be at least 8000 Hz".into());
                }
            }
            "--vocal" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--vocal requires a value".to_string())?;
                vocal_percent = value
                    .parse::<u8>()
                    .map_err(|_| format!("invalid vocal percentage: {value}"))?;
                if vocal_percent > 100 {
                    return Err("vocal percentage must be between 0 and 100".into());
                }
            }
            "--profile" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                profile = match value.as_str() {
                    "music-preservation" => ClassicSuppressionProfile::MusicPreservation,
                    "balanced" => ClassicSuppressionProfile::Balanced,
                    _ => return Err(format!("invalid profile: {value}")),
                };
            }
            "-h" | "--help" => return Err(usage().into()),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let vocal_level = VocalLevel::new(vocal_percent as f32 / 100.0)
        .map_err(str::to_string)?;
    Ok(Config {
        sample_rate,
        vocal_level,
        profile,
    })
}

fn decode_f32le(bytes: &[u8]) -> Vec<f32> {
    let mut samples = Vec::with_capacity(bytes.len() / 4);
    for sample in bytes.chunks_exact(4) {
        samples.push(f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]));
    }
    samples
}

fn write_after_latency(
    samples: &[f32],
    skip_frames: &mut usize,
    output: &mut impl Write,
) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(samples.len() * 4);
    for frame in samples.chunks_exact(2) {
        if *skip_frames > 0 {
            *skip_frames -= 1;
            continue;
        }
        bytes.extend_from_slice(&frame[0].to_le_bytes());
        bytes.extend_from_slice(&frame[1].to_le_bytes());
    }
    output.write_all(&bytes)
}

fn run(config: Config) -> Result<(), String> {
    let mut processor = SpectralCenterSuppressor::new(
        config.sample_rate,
        config.vocal_level,
        config.profile,
    );
    let latency = processor.latency_frames();
    let mut skip_frames = latency;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let mut read_buffer = [0_u8; READ_BYTES];
    let mut pending = Vec::<u8>::new();

    loop {
        let read = input
            .read(&mut read_buffer)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        if read == 0 {
            break;
        }

        pending.extend_from_slice(&read_buffer[..read]);
        let complete_bytes = pending.len() / FRAME_BYTES * FRAME_BYTES;
        if complete_bytes == 0 {
            continue;
        }

        let tail = pending.split_off(complete_bytes);
        let mut samples = decode_f32le(&pending);
        processor.process_stereo_interleaved(&mut samples);
        write_after_latency(&samples, &mut skip_frames, &mut output)
            .map_err(|error| format!("failed to write stdout: {error}"))?;
        pending = tail;
    }

    if !pending.is_empty() {
        return Err(format!(
            "stdin ended with {} trailing byte(s); stereo f32le requires complete 8-byte frames",
            pending.len()
        ));
    }

    let mut flush = vec![0.0_f32; latency * 2];
    processor.process_stereo_interleaved(&mut flush);
    write_after_latency(&flush, &mut skip_frames, &mut output)
        .map_err(|error| format!("failed to flush stdout: {error}"))?;
    output
        .flush()
        .map_err(|error| format!("failed to flush stdout: {error}"))?;
    Ok(())
}

fn main() {
    let config = match parse_args() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            if message != usage() {
                eprintln!("{}", usage());
            }
            std::process::exit(if message == usage() { 0 } else { 2 });
        }
    };

    if let Err(error) = run(config) {
        eprintln!("classic_dsp_raw: {error}");
        std::process::exit(1);
    }
}
