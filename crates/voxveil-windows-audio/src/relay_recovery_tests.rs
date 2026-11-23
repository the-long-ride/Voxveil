use super::*;
use crate::{
    relay_buffer::CaptureTimeline,
    relay_engine::{RelayCommand, RelayHandle, RelaySpec},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use voxveil_model_api::{
    SeparationCapabilities, SeparationEngine, SeparationError, SeparationReport, StemOutput,
};

struct Delayed {
    samples: VecDeque<f32>,
    processed: Arc<AtomicUsize>,
    alive: Arc<AtomicUsize>,
}
impl Drop for Delayed {
    fn drop(&mut self) {
        self.alive.fetch_sub(1, Ordering::SeqCst);
    }
}
impl SeparationEngine for Delayed {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError> {
        self.samples.extend(input);
        for i in 0..input.len() {
            let sample = self.samples.pop_front().unwrap();
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

#[derive(Clone, Default)]
struct Engines {
    created: Arc<AtomicUsize>,
    processed: Arc<AtomicUsize>,
    alive: Arc<AtomicUsize>,
}
fn processor(engines: Engines) -> RelayProcessor {
    let mut processor = RelayProcessor::new(
        100,
        0,
        ClassicSuppressionProfile::default(),
        crate::RelayAiOptions::default(),
    );
    processor.ai_factory = Some(Box::new(move || {
        engines.created.fetch_add(1, Ordering::SeqCst);
        engines.alive.fetch_add(1, Ordering::SeqCst);
        LiveAi::with_engine(
            100,
            Box::new(Delayed {
                samples: VecDeque::from(vec![0.0; 8]),
                processed: Arc::clone(&engines.processed),
                alive: Arc::clone(&engines.alive),
            }),
        )
    }));
    processor.ai = Some(processor.ai_factory.as_ref().unwrap()());
    processor.status = SystemAiStatus::Buffering;
    processor
}
fn wait_for(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !condition() {
        assert!(Instant::now() < deadline, "fake engine did not settle");
        std::thread::yield_now();
    }
}
fn sample(processor: &mut RelayProcessor, engines: &Engines, input: f32) -> f32 {
    let wanted = engines.processed.load(Ordering::Acquire) + 1;
    let mut pcm: Vec<u8> = [input, input]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    processor.process(&mut pcm, false).unwrap();
    wait_for(|| engines.processed.load(Ordering::Acquire) >= wanted);
    f32::from_le_bytes(pcm[..4].try_into().unwrap())
}

#[test]
fn repeated_off_on_while_source_is_playing_starts_clean_and_retries_ai() {
    for origin in [1_000_000, 2_000_000, 50] {
        let engines = Engines::default();
        let counters = engines.clone();
        let (done, complete) = mpsc::channel();
        let mut relay = RelayHandle::spawn_with_worker(
            RelaySpec {
                source_endpoint_id: "default".into(),
                physical_output_endpoint_id: "headset".into(),
            },
            0,
            move |_, _, commands, state| {
                let mut processor = processor(counters.clone());
                let mut timeline = CaptureTimeline::default();
                for n in 0..100 {
                    let update = processor.capture_packet(
                        &mut timeline,
                        origin + n,
                        1,
                        n == 0,
                        n == 0 || n == 80,
                        false,
                    );
                    assert!(!update.resync_ai);
                    let value = sample(&mut processor, &counters, 0.8);
                    if n >= 54 {
                        assert!((value - 0.2).abs() < 1e-6);
                    }
                }
                assert_eq!(processor.status, SystemAiStatus::Running);
                assert_eq!(counters.created.load(Ordering::SeqCst), 1);
                processor.fallback("simulated inference deadline", SystemAiStatus::FellBehind);
                *state.lock().unwrap() = processor.runtime_state();
                done.send(()).unwrap();
                while !matches!(commands.recv(), Ok(RelayCommand::Stop) | Err(_)) {}
            },
        )
        .unwrap();
        complete.recv_timeout(Duration::from_secs(3)).unwrap();
        relay.stop().unwrap();
        wait_for(|| engines.alive.load(Ordering::SeqCst) == 0);
    }
}

#[test]
fn real_discontinuity_rebuffers_and_recovers_without_old_stems_or_leaked_workers() {
    let engines = Engines::default();
    let mut processor = processor(engines.clone());
    let mut timeline = CaptureTimeline::default();
    for n in 0..100 {
        processor.capture_packet(&mut timeline, n, 1, n == 0, false, false);
        sample(&mut processor, &engines, 0.4);
    }
    assert_eq!(processor.status, SystemAiStatus::Running);
    let update = processor.capture_packet(&mut timeline, 100, 1, true, false, false);
    assert!(update.resync_ai);
    assert_eq!(processor.status, SystemAiStatus::Buffering);
    assert_eq!(engines.created.load(Ordering::SeqCst), 2);
    for n in 0..300 {
        if n != 0 {
            processor.capture_packet(&mut timeline, 100 + n, 1, false, false, false);
        }
        let value = sample(&mut processor, &engines, 0.8);
        if n >= 256 {
            assert!((value - 0.2).abs() < 1e-6, "old stems survived resync");
        }
    }
    assert_eq!(processor.status, SystemAiStatus::Running);
    processor.fallback("simulated deadline", SystemAiStatus::FellBehind);
    processor.discontinuity("a subsequent real capture discontinuity");
    assert_eq!(processor.status, SystemAiStatus::Buffering);
    assert_eq!(engines.created.load(Ordering::SeqCst), 3);
    drop(processor);
    wait_for(|| engines.alive.load(Ordering::SeqCst) == 0);
}

#[test]
fn opted_out_dsp_does_not_start_ai_on_discontinuity() {
    let mut processor = RelayProcessor::new(
        48000,
        0,
        ClassicSuppressionProfile::default(),
        crate::RelayAiOptions::default(),
    );
    processor.discontinuity("device gap");
    assert!(processor.ai.is_none());
    assert_eq!(processor.status, SystemAiStatus::Off);
}
