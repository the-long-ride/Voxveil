use super::*;
use crate::{
    relay_buffer::{FRAME_BYTES, MAX_CAPTURE_FRAMES, RelayBuffer},
    relay_idle::IdleCapture,
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use voxveil_model_api::{
    SeparationCapabilities, SeparationEngine, SeparationError, SeparationReport, StemOutput,
};

struct Delayed {
    pcm: VecDeque<f32>,
    processed: Arc<AtomicUsize>,
}
impl SeparationEngine for Delayed {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError> {
        self.pcm.extend(input);
        for i in 0..input.len() {
            let sample = self.pcm.pop_front().unwrap();
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
        1200
    }
    fn capabilities(&self) -> SeparationCapabilities {
        SeparationCapabilities {
            stereo: true,
            streaming: true,
            commercial_redistribution_cleared: true,
        }
    }
}

fn settle(processed: &AtomicUsize, wanted: usize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while processed.load(Ordering::Acquire) < wanted {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

fn short_clips_survive_packetless_silence(ai: bool) {
    let rate = 100;
    let mut processor = RelayProcessor::new(
        rate,
        100,
        ClassicSuppressionProfile::default(),
        crate::RelayAiOptions::default(),
    );
    let processed = Arc::new(AtomicUsize::new(0));
    if ai {
        processor.ai = Some(LiveAi::with_engine(
            rate,
            Box::new(Delayed {
                pcm: VecDeque::from(vec![0.0; 2400]),
                processed: Arc::clone(&processed),
            }),
        ));
        processor.status = SystemAiStatus::Buffering;
    }
    let mut queue = RelayBuffer::new(10, rate);
    let mut device = VecDeque::from(vec![0.0; 10]);
    let start = Instant::now();
    let mut idle = IdleCapture::new(start);
    let mut buffer = vec![0; MAX_CAPTURE_FRAMES * FRAME_BYTES];
    let mut output = Vec::new();
    let mut submitted = 0;
    for frame in 0..3000 {
        let now = start + Duration::from_millis(frame * 10);
        if frame == 0 || frame == 1500 {
            assert_eq!(idle.captured(now), frame != 0);
            let mut pcm: Vec<u8> = [0.5f32, -0.5, 0.25, -0.25]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect();
            processor.process(&mut pcm, false).unwrap();
            queue.push_capture(&pcm);
            submitted += 2;
        }
        output.push(device.pop_front().unwrap());
        submitted += processor
            .fill_idle(&mut idle, now, &mut queue, 9, true, &mut buffer)
            .unwrap();
        if ai {
            settle(&processed, submitted);
            assert_ne!(processor.status, SystemAiStatus::FellBehind);
        }
        let bytes = queue.render_queue();
        let sample = if bytes.is_empty() {
            0.0
        } else {
            let frame: Vec<u8> = bytes.drain(..FRAME_BYTES).collect();
            f32::from_le_bytes(frame[..4].try_into().unwrap())
        };
        device.push_back(sample);
    }
    let audible: Vec<(usize, f32)> = output
        .into_iter()
        .enumerate()
        .filter(|(_, sample)| sample.abs() > 0.01)
        .collect();
    assert_eq!(
        audible.len(),
        4,
        "both two-frame clips must play completely"
    );
    for (i, (_, sample)) in audible.iter().enumerate() {
        assert!((sample - [0.5, 0.25][i % 2]).abs() < 1e-5);
    }
    assert_eq!(
        audible[2].0 - audible[0].0,
        1500,
        "resuming must not duplicate idle silence"
    );
    if ai {
        assert!((1250..1300).contains(&audible[0].0));
    }
}

#[test]
fn short_ai_clips_play_at_fixed_delay_during_idle_and_after_resume() {
    short_clips_survive_packetless_silence(true);
}

#[test]
fn idle_silence_also_flushes_the_dsp_tail() {
    short_clips_survive_packetless_silence(false);
}

#[test]
fn idle_top_up_is_bounded_and_does_not_change_the_reserve_target() {
    let mut queue = RelayBuffer::new(4800, 48000);
    assert_eq!(queue.idle_fill_frames(4800), 0);
    queue.render_queue().clear();
    assert_eq!(queue.idle_fill_frames(0), MAX_CAPTURE_FRAMES);
}
