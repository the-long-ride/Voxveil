use super::*;
use crate::relay_engine::{RelayCommand, RelayHandle, RelayRuntimeState, RelaySpec};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use voxveil_model_api::{
    SeparationCapabilities, SeparationEngine, SeparationError, SeparationReport, StemOutput,
};

struct Fake(Arc<AtomicUsize>);
impl Drop for Fake {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl SeparationEngine for Fake {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError> {
        output.vocals.copy_from_slice(input);
        output.accompaniment.fill(0.0);
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

fn start(options: &RelayAiOptions, live: Arc<AtomicUsize>) -> RelayHandle {
    let ai = options.status == SystemAiStatus::Buffering;
    let (started, ready) = mpsc::channel();
    let handle = RelayHandle::spawn_with_worker(
        RelaySpec {
            source_endpoint_id: "source".into(),
            physical_output_endpoint_id: "sink".into(),
        },
        50,
        move |_, _, commands, state| {
            let _engine = ai.then(|| {
                live.fetch_add(1, Ordering::SeqCst);
                voxveil_ai::live::LiveAi::with_engine(20, Box::new(Fake(live)))
            });
            *state.lock().unwrap() = RelayRuntimeState::Running;
            started.send(()).unwrap();
            while !matches!(commands.recv(), Ok(RelayCommand::Stop) | Err(_)) {}
        },
    )
    .unwrap();
    ready
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    handle
}

#[test]
fn toggles_and_engine_changes_restart_and_release_the_previous_fake_engine() {
    let live = Arc::new(AtomicUsize::new(0));
    let mut options = RelayAiOptions::default();
    let mut relay = start(&options, Arc::clone(&live));
    for status in [
        SystemAiStatus::Buffering,
        SystemAiStatus::Off,
        SystemAiStatus::Buffering,
    ] {
        let next = RelayAiOptions {
            status,
            ..options.clone()
        };
        assert!(options.update(next.clone(), true));
        relay.stop().unwrap();
        relay = start(&next, Arc::clone(&live));
        assert!(!options.update(next, true));
    }
    let next = RelayAiOptions {
        engine: ProcessingEngineKind::Ai,
        ..options.clone()
    };
    assert!(options.update(next, true));
    relay.stop().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while live.load(Ordering::SeqCst) != 0 && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert_eq!(live.load(Ordering::SeqCst), 0);
    assert!(!options.update(RelayAiOptions::default(), false));
}
