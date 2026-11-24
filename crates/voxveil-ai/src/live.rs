use crate::{
    AiEngine,
    player::{AiPlaybackConfig, mix_sample},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use voxveil_model_api::{SeparationEngine, StemOutput};

#[derive(Debug)]
pub enum LiveError {
    Behind(&'static str),
    Backend(String),
}
impl std::fmt::Display for LiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Behind(reason) => f.write_str(reason),
            Self::Backend(reason) => f.write_str(reason),
        }
    }
}

pub struct LiveAi {
    input: SyncSender<Vec<f32>>,
    output: Receiver<Result<Vec<[f32; 2]>, String>>,
    cancel: Arc<AtomicBool>,
    pending: Arc<AtomicUsize>,
    worker: Option<JoinHandle<()>>,
    stems: VecDeque<[f32; 2]>,
    delay: usize,
    elapsed: usize,
    limit: usize,
}

impl LiveAi {
    pub fn with_engine(sample_rate: u32, engine: Box<dyn SeparationEngine>) -> Self {
        Self::spawn(sample_rate, engine.latency_frames(), move || Ok(engine))
    }
    pub fn new(config: AiPlaybackConfig, sample_rate: u32) -> Self {
        let latency = config.model.latency_frames(sample_rate);
        Self::spawn(sample_rate, latency, move || {
            AiEngine::load(
                config.model,
                sample_rate,
                &config.directory,
                &config.runtime,
            )
            .map(|engine| Box::new(engine) as Box<dyn SeparationEngine>)
            .map_err(|e| format!("AI runtime/model load failed: {e:?}"))
        })
    }

    fn spawn(
        rate: u32,
        latency: usize,
        load: impl FnOnce() -> Result<Box<dyn SeparationEngine>, String> + Send + 'static,
    ) -> Self {
        let (input, receiver) = mpsc::sync_channel(256);
        let (sender, output) = mpsc::sync_channel(256);
        let cancel = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&cancel);
        let p = Arc::clone(&pending);
        let worker = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut engine = load()?;
                let mut skip = latency * 2;
                while !c.load(Ordering::Acquire) {
                    let pcm: Vec<f32> = match receiver.recv_timeout(Duration::from_millis(5)) {
                        Ok(pcm) => pcm,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    p.fetch_sub(pcm.len() / 2, Ordering::AcqRel);
                    let mut vocals = vec![0.0; pcm.len()];
                    let mut accompaniment = vocals.clone();
                    let report = engine
                        .process(
                            &pcm,
                            &mut StemOutput {
                                vocals: &mut vocals,
                                accompaniment: &mut accompaniment,
                            },
                        )
                        .map_err(|e| format!("AI inference failed: {e:?}"))?;
                    if report.consumed_frames * 2 != pcm.len()
                        || report.produced_frames * 2 != pcm.len()
                    {
                        return Err("AI returned an incomplete capture block".into());
                    }
                    let drop = skip.min(pcm.len());
                    skip -= drop;
                    if drop < pcm.len() {
                        let stems = vocals[drop..]
                            .iter()
                            .zip(&accompaniment[drop..])
                            .map(|(v, a)| [*v, *a])
                            .collect();
                        if !send(&sender, Ok(stems), &c) {
                            break;
                        }
                    }
                }
                Ok::<_, String>(())
            }));
            let error = match result {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(e),
                Err(_) => Some("AI capture worker panicked".into()),
            };
            if let Some(error) = error {
                send(&sender, Err(error), &c);
            }
        });
        Self {
            input,
            output,
            cancel,
            pending,
            worker: Some(worker),
            stems: VecDeque::new(),
            delay: latency + rate as usize / 2,
            elapsed: 0,
            limit: rate as usize * 2,
        }
    }

    pub fn process(&mut self, pcm: &[f32], level: u8) -> Result<(Vec<f32>, bool), LiveError> {
        if pcm.len() % 2 != 0 {
            return Err(LiveError::Backend("AI capture is not stereo".into()));
        }
        let frames = pcm.len() / 2;
        self.collect()?;
        if self.pending.fetch_add(frames, Ordering::AcqRel) + frames > self.limit {
            self.pending.fetch_sub(frames, Ordering::AcqRel);
            return Err(LiveError::Behind("AI capture backlog exceeded two seconds"));
        }
        if !pcm.is_empty() {
            if self.input.try_send(pcm.to_vec()).is_err() {
                self.pending.fetch_sub(frames, Ordering::AcqRel);
                self.collect()?;
                return Err(LiveError::Behind(
                    "AI capture worker stopped accepting audio",
                ));
            }
        }
        self.collect()?;
        let silence = self.delay.saturating_sub(self.elapsed).min(frames);
        let needed = (frames - silence) * 2;
        if self.stems.len() < needed {
            return Err(LiveError::Behind(
                "AI missed its fixed-delay inference deadline",
            ));
        }
        let mut mixed = vec![0.0; silence * 2];
        mixed.extend(
            self.stems
                .drain(..needed)
                .map(|[v, a]| mix_sample(v, a, level)),
        );
        self.elapsed = self.elapsed.saturating_add(frames);
        Ok((mixed, self.elapsed <= self.delay))
    }

    fn collect(&mut self) -> Result<(), LiveError> {
        loop {
            match self.output.try_recv() {
                Ok(Ok(stems)) => {
                    if self.stems.len() + stems.len() > self.limit * 2 {
                        return Err(LiveError::Behind("AI output backlog exceeded two seconds"));
                    }
                    self.stems.extend(stems);
                }
                Ok(Err(error)) => return Err(LiveError::Backend(error)),
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err(LiveError::Backend("AI capture worker exited".into()));
                }
            }
        }
    }
}

fn send(
    sender: &SyncSender<Result<Vec<[f32; 2]>, String>>,
    mut value: Result<Vec<[f32; 2]>, String>,
    cancel: &AtomicBool,
) -> bool {
    while !cancel.load(Ordering::Acquire) {
        match sender.try_send(value) {
            Ok(()) => return true,
            Err(TrySendError::Disconnected(_)) => return false,
            Err(TrySendError::Full(v)) => {
                value = v;
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
    false
}
impl Drop for LiveAi {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        drop(self.worker.take());
    }
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
