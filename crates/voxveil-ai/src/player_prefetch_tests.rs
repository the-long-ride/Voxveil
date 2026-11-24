use super::super::{FileEngine, mix_sample, preparation::produce_into};
use super::*;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use voxveil_model_api::{
    SeparationCapabilities, SeparationEngine, SeparationError, SeparationReport, StemOutput,
};

struct Listener {
    cache: Arc<Mutex<Cache>>,
    clock: Arc<AtomicU64>,
    queue: VecDeque<(f32, f32)>,
    started: Option<u64>,
    stalls: usize,
    tail_stalls: usize,
    stalled: bool,
    eof: bool,
    frames: usize,
}
impl Listener {
    fn advance(&mut self, ms: u64) {
        for _ in 0..ms / 20 {
            let now = self.clock.fetch_add(20, Ordering::SeqCst) + 20;
            while self.queue.len() < 34 {
                match self.cache.lock().unwrap().poll().unwrap() {
                    Some(Prepared::Stems {
                        vocals,
                        accompaniment,
                    }) => {
                        self.queue.extend(vocals.into_iter().zip(accompaniment));
                    }
                    Some(Prepared::End) => {
                        self.eof = true;
                        break;
                    }
                    None => break,
                    _ => unreachable!(),
                }
            }
            if !self.queue.is_empty() {
                self.started.get_or_insert(now);
            }
            if self.started.is_none() {
                continue;
            }
            for _ in 0..4 {
                if let Some((v, a)) = self.queue.pop_front() {
                    self.stalled = false;
                    let level = ((now / 200) % 3 * 50) as u8;
                    assert_eq!(mix_sample(v, a, level), 0.25 + 0.75 * level as f32 / 100.0);
                    self.frames += 1;
                } else if !self.eof && !self.stalled {
                    self.stalls += 1;
                    self.stalled = true;
                }
            }
        }
    }
}
struct Slow {
    listener: Arc<Mutex<Listener>>,
    pending: usize,
    delay: VecDeque<f32>,
}
impl Slow {
    fn compute(&self) {
        self.listener.lock().unwrap().advance(7200);
    }
}
impl SeparationEngine for Slow {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError> {
        self.pending += input.len() / 2;
        if self.pending >= 600 {
            self.pending -= 600;
            self.compute();
        }
        self.delay.extend(input);
        for i in 0..input.len() {
            let value = self.delay.pop_front().unwrap();
            output.vocals[i] = value * 0.75;
            output.accompaniment[i] = value * 0.25;
        }
        Ok(SeparationReport {
            consumed_frames: input.len() / 2,
            produced_frames: input.len() / 2,
        })
    }
    fn latency_frames(&self) -> usize {
        600
    }
    fn capabilities(&self) -> SeparationCapabilities {
        SeparationCapabilities {
            stereo: true,
            streaming: true,
            commercial_redistribution_cleared: true,
        }
    }
}
impl FileEngine for Slow {
    fn drain(&mut self) -> Result<(Vec<f32>, Vec<f32>), SeparationError> {
        if self.pending > 0 {
            let before = self.listener.lock().unwrap().stalls;
            self.compute();
            self.compute();
            let mut listener = self.listener.lock().unwrap();
            listener.tail_stalls += listener.stalls - before;
        }
        let values: Vec<_> = self.delay.drain(..).collect();
        Ok((
            values.iter().map(|v| v * 0.75).collect(),
            values.iter().map(|v| v * 0.25).collect(),
        ))
    }
}
fn simulate(total: Option<u64>, old_streaming: bool) -> (u64, usize, usize, usize) {
    let clock = Arc::new(AtomicU64::new(0));
    let time = clock.clone();
    let cache = Arc::new(Mutex::new(Cache::with_clock(Box::new(move || {
        Duration::from_millis(time.load(Ordering::SeqCst))
    }))));
    cache.lock().unwrap().configure(100, total, 600);
    cache.lock().unwrap().started = old_streaming;
    let listener = Arc::new(Mutex::new(Listener {
        cache: cache.clone(),
        clock,
        queue: VecDeque::new(),
        started: None,
        stalls: 0,
        tail_stalls: 0,
        stalled: false,
        eof: false,
        frames: 0,
    }));
    let mut engine = Slow {
        listener: listener.clone(),
        pending: 0,
        delay: VecDeque::from(vec![0.0; 1200]),
    };
    let mut frames = 0;
    produce_into(
        &mut engine,
        |pcm| {
            let next = (2900 - frames).min(20);
            pcm.clear();
            pcm.resize(next * 2, 1.0);
            frames += next;
            Ok(next)
        },
        &mut Writer(cache.clone()),
        &AtomicBool::new(false),
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut listener = listener.lock().unwrap();
    listener.advance(40000);
    assert!(listener.eof);
    (
        listener.started.unwrap(),
        listener.stalls,
        listener.frames / 2,
        listener.tail_stalls,
    )
}
#[test]
fn rtf_1_2_plays_the_whole_track_without_stalls_after_one_upfront_wait() {
    let (_, old_stalls, _, old_tail_stalls) = simulate(Some(2900), true);
    assert!(
        old_stalls >= 3,
        "control must reproduce repeated streaming stalls"
    );
    assert!(
        old_tail_stalls > 0,
        "control must exhaust the reserve during the two-chunk tail flush"
    );
    let (start, stalls, frames, tail_stalls) = simulate(Some(2900), false);
    assert!(
        start > 14000 && start < 30000,
        "wait should be upfront, not whole-file preparation: {start}"
    );
    assert_eq!(tail_stalls, 0);
    assert_eq!(stalls, 0);
    assert_eq!(frames, 2900);
}
#[test]
fn unknown_duration_waits_for_eof_then_plays_without_stalls() {
    let (start, stalls, frames, tail_stalls) = simulate(None, false);
    assert!(start >= 43200);
    assert_eq!((stalls, frames), (0, 2900));
    assert_eq!(tail_stalls, 0);
}
#[test]
fn seek_discards_measurements_and_recalculates_the_remaining_wait() {
    let mut first = Prefetch::new(100, Some(2900), 600);
    first.start(Duration::ZERO);
    first.finish(Duration::from_millis(7200), 600);
    let old_wait = first
        .wait_ms(Duration::from_millis(7200), 580, false)
        .unwrap();
    let mut seek = Prefetch::new(100, Some(900), 600);
    assert_eq!(seek.wait_ms(Duration::ZERO, 0, false), None);
    seek.start(Duration::ZERO);
    seek.finish(Duration::from_millis(7200), 600);
    assert!(
        seek.wait_ms(Duration::from_millis(7200), 580, false)
            .unwrap()
            < old_wait
    );
    assert_eq!(
        seek.wait_ms(Duration::from_millis(7200), 900, true),
        Some(0)
    );
}
#[test]
fn empty_cache_finishes_and_storage_errors_are_reported() {
    let mut empty = Cache::new();
    empty.configure(100, Some(0), 600);
    empty.push(Prepared::End).unwrap();
    assert!(matches!(empty.poll().unwrap(), Some(Prepared::End)));
    assert!(empty.poll().unwrap().is_none());
    let mut broken = Cache::new();
    broken.file = Some(
        File::open(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap(),
    );
    let error = broken
        .push(Prepared::Stems {
            vocals: vec![1.0; 2],
            accompaniment: vec![1.0; 2],
        })
        .unwrap_err();
    assert!(error.contains("write failed"));
}

#[test]
fn cancelling_a_paused_cache_detaches_inflight_work_and_eventually_releases_the_file() {
    use super::super::preparation::Sink;
    use super::super::{StemProducer, wait};
    use std::sync::mpsc;
    let cache = Arc::new(Mutex::new(Cache::new()));
    cache.lock().unwrap().configure(100, Some(100), 20);
    let weak = Arc::downgrade(&cache);
    let writer = cache.clone();
    let (entered, ready) = mpsc::sync_channel(1);
    let (release, resume) = mpsc::sync_channel(1);
    let mut producer = StemProducer::start(move |_, cancel, paused| {
        let mut writer = Writer(writer);
        writer.work_start()?;
        writer.emit(Prepared::Stems {
            vocals: vec![0.75; 40],
            accompaniment: vec![0.25; 40],
        })?;
        writer.work_done(20)?;
        entered.send(()).unwrap();
        resume.recv_timeout(Duration::from_secs(2)).unwrap();
        wait(cancel, paused)
    });
    producer.cache = Some(cache);
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    producer.pause(true);
    let start = Instant::now();
    producer.stop();
    assert!(start.elapsed() < Duration::from_millis(100));
    assert!(weak.upgrade().is_some());
    release.send(()).unwrap();
    while weak.upgrade().is_some() {
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::yield_now();
    }
}
