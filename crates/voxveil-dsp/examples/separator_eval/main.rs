mod audio;
mod metrics;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::{SeparatorEvalTap, SeparatorParams, SpectralCenterSuppressor, VocalSeparator};
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

use audio::{Stems, frame_at, load_stems, write_wav};
use metrics::{Row, energy, error_energy, median, ratio_db};

const USAGE: &str = "usage: separator_eval --pair VOCAL,INSTRUMENTAL [--pair ...] [--sweep key=v1,v2,...] [--render DIR] [--csv FILE]";

struct Config {
    pairs: Vec<(PathBuf, PathBuf)>,
    sweep: Option<(String, Vec<f32>)>,
    render: Option<PathBuf>,
    csv: Option<PathBuf>,
}

fn parse_args() -> Result<Config, String> {
    let mut config = Config { pairs: Vec::new(), sweep: None, render: None, csv: None };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} requires a value"));
        match arg.as_str() {
            "--pair" => {
                let raw = value()?;
                let (vocal, instrumental) =
                    raw.split_once(',').ok_or("--pair expects VOCAL,INSTRUMENTAL")?;
                config.pairs.push((vocal.into(), instrumental.into()));
            }
            "--sweep" => {
                let raw = value()?;
                let (key, values) = raw.split_once('=').ok_or("--sweep expects key=v1,v2")?;
                let values = values
                    .split(',')
                    .map(|v| v.parse::<f32>().map_err(|_| format!("bad sweep value {v}")))
                    .collect::<Result<Vec<_>, _>>()?;
                SeparatorParams::default().set(key, values[0])?;
                config.sweep = Some((key.to_string(), values));
            }
            "--render" => config.render = Some(value()?.into()),
            "--csv" => config.csv = Some(value()?.into()),
            _ => return Err(format!("unknown argument: {arg}\n{USAGE}")),
        }
    }
    if config.pairs.is_empty() {
        return Err(USAGE.into());
    }
    Ok(config)
}

fn render(processor: &mut dyn AudioProcessor, input: &[f32]) -> (Vec<f32>, f64) {
    let latency = processor.latency_frames();
    let mut buffer = input.to_vec();
    buffer.extend(std::iter::repeat_n(0.0, latency * 2));
    let start = Instant::now();
    for chunk in buffer.chunks_mut(1_024) {
        processor.process_stereo_interleaved(chunk);
    }
    let seconds = start.elapsed().as_secs_f64();
    buffer.drain(..latency * 2);
    (buffer, seconds)
}

fn isolation_outputs(stems: &Stems, params: SeparatorParams) -> [Vec<f32>; 3] {
    let level = VocalLevel::new(0.0).unwrap();
    let mut separator = VocalSeparator::with_params(stems.sample_rate, level, params);
    let mut tap = SeparatorEvalTap::new(&separator);
    let latency = separator.latency_frames();
    let mut outputs = [Vec::new(), Vec::new(), Vec::new()];
    for n in 0..stems.frames() + latency {
        let frames = tap.process_frame(
            &mut separator,
            frame_at(&stems.vocal, n),
            frame_at(&stems.instrumental, n),
        );
        if n >= latency {
            for (output, frame) in outputs.iter_mut().zip(frames) {
                output.extend_from_slice(&frame);
            }
        }
    }
    outputs
}

fn evaluate(
    stems: &Stems,
    variant: &str,
    params: Option<SeparatorParams>,
    render_dir: Option<&PathBuf>,
) -> Result<Row, String> {
    let level = VocalLevel::new(0.0).unwrap();
    let mix = stems.mix();
    let inst_energy = energy(&stems.instrumental);
    let (output, seconds, vocal_reduction_db, inst_damage_sdr) = match params {
        None => {
            let profile = match variant {
                "music-preservation" => ClassicSuppressionProfile::MusicPreservation,
                "balanced" => ClassicSuppressionProfile::Balanced,
                _ => ClassicSuppressionProfile::Strong,
            };
            let mut legacy = SpectralCenterSuppressor::new(stems.sample_rate, level, profile);
            let (output, seconds) = render(&mut legacy, &mix);
            (output, seconds, None, None)
        }
        Some(params) => {
            let mut separator = VocalSeparator::with_params(stems.sample_rate, level, params);
            let (_, seconds) = render(&mut separator, &mix);
            let [output, vocal_out, inst_out] = isolation_outputs(stems, params);
            let reduction = ratio_db(energy(&stems.vocal), energy(&vocal_out));
            let damage = ratio_db(inst_energy, error_energy(&inst_out, &stems.instrumental));
            (output, seconds, Some(reduction), Some(damage))
        }
    };
    if let Some(dir) = render_dir {
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let path = dir.join(format!("{}__{variant}.wav", stems.name));
        write_wav(&path, stems.sample_rate, &output)?;
    }
    Ok(Row {
        song: stems.name.clone(),
        variant: variant.to_string(),
        sdr_inst: ratio_db(inst_energy, error_energy(&output, &stems.instrumental)),
        vocal_reduction_db,
        inst_damage_sdr,
        cpu_percent: 100.0 * seconds / stems.duration_seconds(),
    })
}

fn variants(config: &Config) -> Vec<(String, Option<SeparatorParams>)> {
    let mut list: Vec<(String, Option<SeparatorParams>)> =
        ["music-preservation", "balanced", "strong"]
            .iter()
            .map(|name| (name.to_string(), None))
            .collect();
    list.push(("isolation".into(), Some(SeparatorParams::default())));
    if let Some((key, values)) = &config.sweep {
        for value in values {
            let mut params = SeparatorParams::default();
            params.set(key, *value).expect("validated in parse_args");
            list.push((format!("isolation[{key}={value}]"), Some(params)));
        }
    }
    list
}

fn print_medians(rows: &[Row], variants: &[(String, Option<SeparatorParams>)], songs: usize) {
    println!("\nmedian over {songs} song(s):");
    println!(
        "{:<40} {:>9} {:>11} {:>11} {:>7}",
        "variant", "SDR_inst", "vocal_red", "inst_dmg", "cpu%"
    );
    for (variant, _) in variants {
        let pick = |f: &dyn Fn(&Row) -> Option<f64>| {
            let mut values: Vec<f64> =
                rows.iter().filter(|r| &r.variant == variant).filter_map(f).collect();
            if values.is_empty() { f64::NAN } else { median(&mut values) }
        };
        println!(
            "{:<40} {:>9.2} {:>11.2} {:>11.2} {:>7.3}",
            variant,
            pick(&|r| Some(r.sdr_inst)),
            pick(&|r| r.vocal_reduction_db),
            pick(&|r| r.inst_damage_sdr),
            pick(&|r| Some(r.cpu_percent)),
        );
    }
}

fn run(config: Config) -> Result<(), String> {
    let variants = variants(&config);
    let mut rows = Vec::new();
    for (vocal, instrumental) in &config.pairs {
        let stems = load_stems(vocal, instrumental)?;
        eprintln!("{}: {:.1} s @ {} Hz", stems.name, stems.duration_seconds(), stems.sample_rate);
        for (variant, params) in &variants {
            let row = evaluate(&stems, variant, *params, config.render.as_ref())?;
            println!("{}", row.to_csv());
            rows.push(row);
        }
    }
    print_medians(&rows, &variants, config.pairs.len());
    if let Some(path) = &config.csv {
        let body: Vec<String> = std::iter::once(Row::CSV_HEADER.to_string())
            .chain(rows.iter().map(Row::to_csv))
            .collect();
        fs::write(path, body.join("\n") + "\n").map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn main() {
    if let Err(message) = parse_args().and_then(run) {
        eprintln!("separator_eval: {message}");
        std::process::exit(2);
    }
}
