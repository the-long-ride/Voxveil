use super::*;
use voxveil_model_api::{SeparationCapabilities, SeparationError, SeparationReport};

struct Fake {
    delay: VecDeque<f32>,
    processed: Arc<AtomicUsize>,
    gate: Option<Receiver<()>>,
}
impl SeparationEngine for Fake {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError> {
        if let Some(gate) = self.gate.take() {
            gate.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        self.delay.extend(input);
        for i in 0..input.len() {
            let sample = self.delay.pop_front().unwrap();
            output.vocals[i] = sample * 0.75;
            output.accompaniment[i] = sample * 0.25;
        }
        self.processed.fetch_add(input.len() / 2, Ordering::Release);
        Ok(SeparationReport {
            consumed_frames: input.len() / 2,
            produced_frames: input.len() / 2,
        })
    }
    fn latency_frames(&self) -> usize {
        4
    }
    fn capabilities(&self) -> SeparationCapabilities {
        SeparationCapabilities {
            stereo: true,
            streaming: true,
            commercial_redistribution_cleared: true,
        }
    }
}

fn fake(processed: Arc<AtomicUsize>) -> Fake {
    Fake {
        delay: VecDeque::from(vec![0.0; 8]),
        processed,
        gate: None,
    }
}

#[test]
fn fixed_delay_alignment_and_live_mixing_survive_many_model_boundaries() {
    let processed = Arc::new(AtomicUsize::new(0));
    let mut ai = LiveAi::with_engine(20, Box::new(fake(Arc::clone(&processed))));
    assert_eq!(ai.delay, 14);
    assert_eq!(ai.process(&[], 100).unwrap(), (vec![], true));
    for n in 0..100 {
        let level = [0, 50, 100][n % 3];
        let input = n as f32 + 1.0;
        let (output, buffering) = ai.process(&[input, -input], level).unwrap();
        if n < ai.delay {
            assert_eq!(output, vec![0.0, 0.0]);
            assert!(buffering);
        } else {
            let original = (n - ai.delay) as f32 + 1.0;
            let expected = original * (0.25 + 0.75 * level as f32 / 100.0);
            assert_eq!(output, vec![expected, -expected]);
            assert!(!buffering);
        }
        let wanted = (n + 1).saturating_sub(4).min(10) * 2;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while processed.load(Ordering::Acquire) < n + 1 || ai.stems.len() < wanted {
            ai.collect().unwrap();
            assert!(
                std::time::Instant::now() < deadline,
                "fake engine did not finish"
            );
            thread::yield_now();
        }
    }
}

#[test]
fn stalled_inference_reports_a_missed_deadline_instead_of_synthesizing_audio() {
    let processed = Arc::new(AtomicUsize::new(0));
    let (release, gate) = mpsc::channel();
    let mut engine = fake(processed);
    engine.gate = Some(gate);
    let mut ai = LiveAi::with_engine(20, Box::new(engine));
    assert!(ai.process(&[1.0, 1.0], 100).unwrap().1);
    assert!(matches!(
        ai.process(&[1.0; 28], 100),
        Err(LiveError::Behind(_))
    ));
    drop(ai);
    let _ = release.send(());
}

#[test]
fn invalid_input_backpressure_and_load_errors_are_explicit() {
    let mut ai = LiveAi::with_engine(20, Box::new(fake(Arc::new(AtomicUsize::new(0)))));
    assert!(matches!(
        ai.process(&[1.0], 100),
        Err(LiveError::Backend(_))
    ));
    assert!(matches!(
        ai.process(&[0.0; 82], 100),
        Err(LiveError::Behind(_))
    ));
    let mut failed = LiveAi::spawn(20, 4, || Err("missing model".into()));
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Err(error) = failed.collect() {
            assert!(matches!(error, LiveError::Backend(_)));
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        thread::yield_now();
    }
}
