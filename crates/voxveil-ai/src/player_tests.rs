use super::*;
use std::{collections::VecDeque, sync::atomic::AtomicUsize};
use voxveil_model_api::{SeparationCapabilities, SeparationReport};

struct Fake {
    delay: VecDeque<f32>,
    live: Arc<AtomicUsize>,
}
impl Fake {
    fn new(live: Arc<AtomicUsize>) -> Self {
        live.fetch_add(1, Ordering::SeqCst);
        Self {
            delay: VecDeque::from(vec![0.0; 4]),
            live,
        }
    }
}
impl Drop for Fake {
    fn drop(&mut self) {
        self.live.fetch_sub(1, Ordering::SeqCst);
    }
}
impl SeparationEngine for Fake {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<voxveil_model_api::SeparationReport, SeparationError> {
        self.delay.extend(input);
        for i in 0..input.len() {
            let sample = self.delay.pop_front().unwrap();
            output.vocals[i] = sample * 0.75;
            output.accompaniment[i] = sample * 0.25;
        }
        Ok(SeparationReport {
            consumed_frames: input.len() / 2,
            produced_frames: input.len() / 2,
        })
    }
    fn latency_frames(&self) -> usize {
        2
    }
    fn capabilities(&self) -> SeparationCapabilities {
        SeparationCapabilities {
            stereo: true,
            streaming: true,
            commercial_redistribution_cleared: true,
        }
    }
}
impl FileEngine for Fake {
    fn drain(&mut self) -> Result<(Vec<f32>, Vec<f32>), SeparationError> {
        let samples: Vec<_> = self.delay.drain(..).collect();
        Ok((
            samples.iter().map(|x| x * 0.75).collect(),
            samples.iter().map(|x| x * 0.25).collect(),
        ))
    }
}

fn fake_producer(value: f32, live: Arc<AtomicUsize>) -> StemProducer {
    StemProducer::start(move |sender, cancel, paused| {
        let mut engine = Fake::new(live);
        let mut blocks = 0;
        produce(
            &mut engine,
            |pcm| {
                blocks += 1;
                pcm.clear();
                if blocks > 3 {
                    return Ok(0);
                }
                pcm.resize(8, value);
                Ok(4)
            },
            sender,
            cancel,
            paused,
        )
    })
}

#[test]
fn mixing_keeps_buffered_stems_live_at_zero_half_and_full() {
    assert_eq!(mix_sample(0.6, 0.2, 0), 0.2);
    assert_eq!(mix_sample(0.6, 0.2, 50), 0.5);
    assert_eq!(mix_sample(0.6, 0.2, 100), 0.8);
}

#[test]
fn preparation_strips_latency_and_retains_the_complete_tail() {
    let live = Arc::new(AtomicUsize::new(0));
    let mut producer = fake_producer(1.0, Arc::clone(&live));
    let mut output = Vec::new();
    loop {
        match producer
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap()
        {
            Prepared::Stems {
                vocals,
                accompaniment,
            } => output.extend(
                vocals
                    .iter()
                    .zip(accompaniment)
                    .map(|(v, a)| mix_sample(*v, a, 100)),
            ),
            Prepared::End => break,
            _ => unreachable!(),
        }
    }
    producer.stop();
    assert_eq!(output, vec![1.0; 24]);
    assert_eq!(live.load(Ordering::SeqCst), 0);
}

#[test]
fn seek_replaces_old_audio_and_teardown_joins_even_a_full_paused_queue() {
    let live = Arc::new(AtomicUsize::new(0));
    let mut producer = fake_producer(1.0, Arc::clone(&live));
    assert!(
        producer
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_ok()
    );
    producer.pause(true);
    producer.stop();
    assert_eq!(live.load(Ordering::SeqCst), 0);
    let mut restarted = fake_producer(2.0, Arc::clone(&live));
    if let Prepared::Stems {
        vocals,
        accompaniment,
    } = restarted
        .receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap()
    {
        assert!(
            vocals
                .iter()
                .zip(accompaniment)
                .all(|(v, a)| mix_sample(*v, a, 100) == 2.0)
        );
    } else {
        panic!("expected restarted stems");
    }
    restarted.stop();
    assert_eq!(live.load(Ordering::SeqCst), 0);
}

#[test]
fn slow_preparation_is_pending_not_synthetic_silence_and_errors_surface() {
    let mut producer = StemProducer::start(|_, cancel, paused| {
        wait(cancel, paused)?;
        Err("model failure".into())
    });
    assert_eq!(
        producer
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .err(),
        Some("model failure".into())
    );
    producer.stop();
    let live = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(AtomicBool::new(false));
    let gate = Arc::clone(&release);
    let mut slow = StemProducer::start(move |sender, cancel, paused| {
        let mut engine = Fake::new(live);
        produce(&mut engine, |_| {
            while !gate.load(Ordering::Acquire) { wait(cancel, paused)?; thread::sleep(Duration::from_millis(2)); }
            Ok(0)
        }, sender, cancel, paused)
    });
    assert!(matches!(
        slow.receiver.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    release.store(true, Ordering::Release);
    assert!(matches!(slow.receiver.recv_timeout(Duration::from_secs(2)).unwrap().unwrap(), Prepared::End));
    slow.stop();
}
