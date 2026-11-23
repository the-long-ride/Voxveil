//! Bounded, cancellable file prefetch. Both stems remain unmixed until render.
use crate::{AiEngine, Model};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use voxveil_media::MediaDecoder;
use voxveil_model_api::{SeparationEngine, SeparationError};

#[path = "player_cache.rs"]
mod cache;
#[path = "player_prefetch.rs"]
mod prefetch;
#[path = "player_prepare.rs"]
mod preparation;
#[path = "player_temp.rs"]
mod temporary;
use preparation::produce;
#[cfg(test)]
use voxveil_model_api::StemOutput;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiPlaybackConfig {
    pub model: Model,
    pub directory: PathBuf,
    pub runtime: PathBuf,
}

pub trait FileEngine: SeparationEngine {
    fn drain(&mut self) -> Result<(Vec<f32>, Vec<f32>), SeparationError>;
}
impl FileEngine for AiEngine {
    fn drain(&mut self) -> Result<(Vec<f32>, Vec<f32>), SeparationError> {
        self.finish()
    }
}

pub enum Prepared {
    Position(u64),
    Stems {
        vocals: Vec<f32>,
        accompaniment: Vec<f32>,
    },
    End,
}

pub fn mix_sample(vocal: f32, accompaniment: f32, level: u8) -> f32 {
    accompaniment + level.min(100) as f32 / 100.0 * vocal
}

pub struct StemProducer {
    receiver: Receiver<Result<Prepared, String>>,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    cache: Option<Arc<Mutex<cache::Cache>>>,
}

impl StemProducer {
    pub fn spawn(path: PathBuf, position: u64, config: AiPlaybackConfig) -> Self {
        let cache =
            (config.model == Model::MdxInstHq3).then(|| Arc::new(Mutex::new(cache::Cache::new())));
        let writer = cache.clone();
        let mut producer = Self::start(move |sender, cancel, paused| {
            let mut decoder = MediaDecoder::open(&path).map_err(|e| e.to_string())?;
            let actual = decoder.seek(position).map_err(|e| e.to_string())?;
            let mut engine = AiEngine::load(
                config.model,
                decoder.info().sample_rate,
                &config.directory,
                &config.runtime,
            )
            .map_err(|e| format!("AI model could not be loaded: {e:?}"))?;
            send(sender, Ok(Prepared::Position(actual)), cancel, paused)?;
            if let Some(cache) = writer {
                cache
                    .lock()
                    .map_err(|_| "AI cache lock is poisoned")?
                    .configure(
                        decoder.info().sample_rate,
                        decoder
                            .info()
                            .total_frames
                            .map(|total| total.saturating_sub(actual)),
                        engine.latency_frames(),
                    );
                preparation::produce_into(
                    &mut engine,
                    |pcm| decoder.read_frames(8192, pcm).map_err(|e| e.to_string()),
                    &mut cache::Writer(cache),
                    cancel,
                    paused,
                )
            } else {
                produce(
                    &mut engine,
                    |pcm| decoder.read_frames(8192, pcm).map_err(|e| e.to_string()),
                    sender,
                    cancel,
                    paused,
                )
            }
        });
        producer.cache = cache;
        producer
    }

    fn start(
        work: impl FnOnce(
            &SyncSender<Result<Prepared, String>>,
            &AtomicBool,
            &AtomicBool,
        ) -> Result<(), String>
        + Send
        + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(2);
        let cancel = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&cancel);
        let p = Arc::clone(&paused);
        let worker = thread::spawn(move || {
            if c.load(Ordering::Acquire) {
                return;
            }
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&sender, &c, &p)));
            let error = match result {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(e),
                Err(_) => Some("AI preparation worker panicked".into()),
            };
            if let Some(error) = error {
                let _ = send(&sender, Err(error), &c, &p);
            }
        });
        Self {
            receiver,
            cancel,
            paused,
            worker: Some(worker),
            cache: None,
        }
    }

    pub fn pause(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    pub fn poll(&mut self) -> Result<Option<Prepared>, String> {
        match self.receiver.try_recv() {
            Ok(result) => result.map(Some),
            Err(_) if self.cache.is_some() => self
                .cache
                .as_ref()
                .unwrap()
                .lock()
                .map_err(|_| "AI cache lock is poisoned")?
                .poll(),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("AI preparation ended without a complete tail".into())
            }
        }
    }
    pub fn preparing_wait_ms(&self) -> Option<u64> {
        self.cache.as_ref()?.lock().ok()?.wait_ms()
    }

    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        drop(self.worker.take());
        let (_, empty) = mpsc::channel();
        self.receiver = empty;
        self.cache.take();
    }
}
impl Drop for StemProducer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn wait(cancel: &AtomicBool, paused: &AtomicBool) -> Result<(), String> {
    while paused.load(Ordering::Acquire) && !cancel.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(2));
    }
    if cancel.load(Ordering::Acquire) {
        Err("AI preparation cancelled".into())
    } else {
        Ok(())
    }
}

fn send(
    sender: &SyncSender<Result<Prepared, String>>,
    mut message: Result<Prepared, String>,
    cancel: &AtomicBool,
    paused: &AtomicBool,
) -> Result<(), String> {
    loop {
        wait(cancel, paused)?;
        match sender.try_send(message) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Full(value)) => {
                message = value;
                thread::sleep(Duration::from_millis(2));
            }
            Err(TrySendError::Disconnected(_)) => return Err("AI consumer disconnected".into()),
        }
    }
}

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
