use super::*;

struct Identity;
impl ChunkModel for Identity {
    fn run(&mut self, input: &[f32]) -> Result<Vec<f32>, SeparationError> {
        Ok(input.to_vec())
    }
}

struct Failure;
impl ChunkModel for Failure {
    fn run(&mut self, _: &[f32]) -> Result<Vec<f32>, SeparationError> {
        Err(SeparationError::BackendFailure)
    }
}

#[test]
fn crossfade_smooths_disagreement_between_chunks_without_a_gain_bump() {
    struct Alternating(f32);
    impl ChunkModel for Alternating {
        fn run(&mut self, input: &[f32]) -> Result<Vec<f32>, SeparationError> {
            self.0 = -self.0;
            Ok(vec![self.0; input.len()])
        }
    }
    let mut chunks = Chunks::new(128, 32);
    chunks
        .push(&vec![0.0; 1280], &mut Alternating(-1.0))
        .unwrap();
    let audio: Vec<_> = chunks
        .output
        .into_iter()
        .skip(256)
        .map(|stems| stems[1])
        .collect();
    assert!(audio.iter().all(|x| x.abs() <= 1.0));
    assert!(
        audio
            .windows(3)
            .all(|samples| (samples[2] - samples[0]).abs() < 0.1)
    );
    assert!(audio.iter().any(|x| x.abs() < 0.1));
}

fn process(engine: &mut AiEngine, input: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut vocals = vec![0.0; input.len()];
    let mut accompaniment = vocals.clone();
    let report = engine
        .process(
            input,
            &mut StemOutput {
                vocals: &mut vocals,
                accompaniment: &mut accompaniment,
            },
        )
        .unwrap();
    assert_eq!(report.consumed_frames * 2, input.len());
    assert_eq!(report.produced_frames * 2, input.len());
    (vocals, accompaniment)
}

#[test]
fn crossfade_preserves_continuity_length_and_stereo_for_arbitrary_packets() {
    for model in [Model::Spleeter, Model::MdxInstHq3] {
        let mut engine = AiEngine::with_model(model, 48000, Box::new(Identity));
        let latency = engine.latency_frames() * 2;
        let input: Vec<_> = (0..latency * 3 + 318)
            .map(|i| (i as f32 * 0.0123).sin() * 0.25)
            .collect();
        let mut output = Vec::new();
        for packet in input.chunks(998) {
            let (v, a) = process(&mut engine, packet);
            assert!(v.iter().all(|x| x.abs() < 1e-7));
            output.extend(a);
        }
        let (v, a) = engine.finish().unwrap();
        assert!(v.iter().all(|x| x.abs() < 1e-7));
        output.extend(a);
        assert_eq!(output.len(), input.len() + latency);
        assert!(output[..latency].iter().all(|x| *x == 0.0));
        let max_error = output[latency..]
            .iter()
            .zip(input)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_error < 1e-7, "boundary error {max_error}");
        assert_eq!(engine.finish(), Err(SeparationError::BackendFailure));
    }
}

#[test]
fn empty_input_and_finish_are_well_defined() {
    let mut engine = AiEngine::with_model(Model::Spleeter, 44100, Box::new(Identity));
    assert_eq!(process(&mut engine, &[]), (vec![], vec![]));
    let (v, a) = engine.finish().unwrap();
    assert_eq!(a.len(), engine.latency_frames() * 2);
    assert!(v.iter().chain(&a).all(|x| *x == 0.0));
}

#[test]
fn invalid_buffers_and_backend_failures_are_reported() {
    let mut engine = AiEngine::with_model(Model::MdxInstHq3, 44100, Box::new(Failure));
    for input in [vec![0.0], vec![f32::NAN, 0.0], vec![f32::INFINITY, 0.0]] {
        assert_eq!(
            engine.process(
                &input,
                &mut StemOutput {
                    vocals: &mut [0.0; 4],
                    accompaniment: &mut [0.0; 4]
                }
            ),
            Err(SeparationError::InvalidBuffer)
        );
    }
    assert_eq!(
        engine.process(
            &[0.0; 4],
            &mut StemOutput {
                vocals: &mut [],
                accompaniment: &mut [0.0; 4]
            }
        ),
        Err(SeparationError::InvalidBuffer)
    );
    let input = vec![0.0; engine.latency_frames() * 2];
    let mut v = input.clone();
    let mut a = input.clone();
    assert_eq!(
        engine.process(
            &input,
            &mut StemOutput {
                vocals: &mut v,
                accompaniment: &mut a
            }
        ),
        Err(SeparationError::BackendFailure)
    );
    assert_eq!(engine.finish(), Err(SeparationError::BackendFailure));
    assert!(matches!(
        AiEngine::load(
            Model::Spleeter,
            48000,
            Path::new("missing"),
            Path::new("missing")
        ),
        Err(SeparationError::ModelUnavailable)
    ));
    assert!(matches!(
        AiEngine::load(
            Model::Spleeter,
            96000,
            Path::new("missing"),
            Path::new("missing")
        ),
        Err(SeparationError::InvalidBuffer)
    ));
}

#[test]
fn latency_reports_the_full_host_rate_chunk() {
    for rate in [44100, 48000] {
        for (model, seconds) in [(Model::Spleeter, 11.9583), (Model::MdxInstHq3, 5.9211)] {
            let engine = AiEngine::with_model(model, rate, Box::new(Identity));
            assert!((engine.latency_frames() as f64 / rate as f64 - seconds).abs() < 0.001);
            assert_eq!(engine.latency_frames(), model.latency_frames(rate));
            assert!(engine.capabilities().stereo && engine.capabilities().streaming);
        }
    }
}

#[test]
fn fft_resampling_round_trip_preserves_audio_and_duration() {
    let input: Vec<_> = (0..48000)
        .flat_map(|i| {
            let t = i as f32 / 48000.0;
            [
                (t * 997.0 * std::f32::consts::TAU).sin() * 0.5,
                (t * 5011.0 * std::f32::consts::TAU).sin() * 0.25,
            ]
        })
        .collect();
    let native = resample::convert(&input, 48000, 44100, 44100).unwrap();
    let output = resample::convert(&native, 44100, 48000, 48000).unwrap();
    assert_eq!(output.len(), input.len());
    let mut signal = 0.0;
    let mut error = 0.0;
    for (a, b) in input[4096..input.len() - 4096]
        .iter()
        .zip(&output[4096..output.len() - 4096])
    {
        signal += (*a as f64).powi(2);
        error += (*a as f64 - *b as f64).powi(2);
    }
    let snr = 10.0 * (signal / error).log10();
    assert!(snr > 65.0, "round-trip SNR {snr} dB");
    assert!(resample::convert(&[], 48000, 44100, 0).unwrap().is_empty());
}
