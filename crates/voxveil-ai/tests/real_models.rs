use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use voxveil_ai::{AiEngine, Model};
use voxveil_model_api::{SeparationEngine, StemOutput};

fn read(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap();
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().sample_rate, 48000);
    match reader.spec().sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => {
            let scale = (1_u64 << (reader.spec().bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / scale)
                .collect()
        }
    }
}

fn evaluate(model: Model) {
    let root = PathBuf::from(
        std::env::var_os("VOXVEIL_VOCAL_TEST").unwrap_or("/home/ubuntu/voxveil-vocal-test".into()),
    );
    let default = match model {
        Model::Spleeter => root.join("ai/sherpa-onnx-spleeter-2stems-fp16"),
        Model::MdxInstHq3 => root.join("ai"),
    };
    let directory = std::env::var_os("VOXVEIL_AI_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or(default);
    let library = PathBuf::from(
        std::env::var_os("VOXVEIL_ORT_DYLIB")
            .expect("set VOXVEIL_ORT_DYLIB to the absolute ONNX Runtime library path"),
    );
    let input = read(&root.join("mix.wav"));
    let vocal = read(&root.join("vocal3.wav"));
    let backing = read(&root.join("accomp.wav"));
    let mut engine = AiEngine::load(model, 48000, &directory, &library).unwrap();
    let latency = engine.latency_frames() * 2;
    let start = Instant::now();
    let mut result = Vec::new();
    for packet in input.chunks(8192) {
        let mut v = vec![0.0; packet.len()];
        let mut a = v.clone();
        engine
            .process(
                packet,
                &mut StemOutput {
                    vocals: &mut v,
                    accompaniment: &mut a,
                },
            )
            .unwrap();
        result.extend(a);
    }
    result.extend(engine.finish().unwrap().1);
    let elapsed = start.elapsed().as_secs_f64();
    let output = &result[latency..latency + input.len()];
    let range = 48000..input.len() - 48000;
    let (mut vv, mut aa, mut va, mut vy, mut ay) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for i in range.clone() {
        let (v, a, y) = (vocal[i] as f64, backing[i] as f64, output[i] as f64);
        vv += v * v;
        aa += a * a;
        va += v * a;
        vy += v * y;
        ay += a * y;
    }
    let det = vv * aa - va * va;
    let cv = (vy * aa - ay * va) / det;
    let ca = (ay * vv - vy * va) / det;
    let (mut residual, mut energy) = (0.0, 0.0);
    for i in range {
        let y = output[i] as f64;
        residual += (y - cv * vocal[i] as f64 - ca * backing[i] as f64).powi(2);
        energy += y * y;
    }
    let vocal_db = 20.0 * cv.abs().log10();
    let accompaniment_db = 20.0 * ca.abs().log10();
    println!(
        "{model:?}: vocal={vocal_db:.3} dB accompaniment={accompaniment_db:.3} dB residual={:.3} dB RTF={:.3} seconds={elapsed:.3}",
        10.0 * (residual / energy).log10(),
        elapsed / (input.len() as f64 / 96000.0)
    );
    assert!(vocal_db < -20.0);
    assert!(accompaniment_db > -2.0);
}

#[test]
#[ignore = "requires locally downloaded Spleeter weights, ONNX Runtime and CC BY test stems"]
fn spleeter_real_audio() {
    evaluate(Model::Spleeter);
}

#[test]
#[ignore = "requires locally downloaded UVR HQ3 weights, ONNX Runtime and CC BY test stems"]
fn mdx_real_audio() {
    evaluate(Model::MdxInstHq3);
}
