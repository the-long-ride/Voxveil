//! Bounded, cancellable file prefetch. Both stems remain unmixed until render.
use crate::{AiEngine, Model};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use voxveil_media::MediaDecoder;
use voxveil_model_api::{SeparationEngine, SeparationError, StemOutput};

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
    pub receiver: Receiver<Result<Prepared, String>>,
    cancel: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl StemProducer {
    pub fn spawn(path: PathBuf, position: u64, config: AiPlaybackConfig) -> Self {
        Self::start(move |sender, cancel, paused| {
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
            produce(
                &mut engine,
                |pcm| decoder.read_frames(8192, pcm).map_err(|e| e.to_string()),
                sender,
                cancel,
                paused,
            )
        })
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
            if c.load(Ordering::Acquire) { return; }
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
        }
    }

    pub fn pause(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        drop(self.worker.take());
        let (_, empty) = mpsc::channel();
        self.receiver = empty;
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

fn produce(
    engine: &mut dyn FileEngine,
    mut read: impl FnMut(&mut Vec<f32>) -> Result<usize, String>,
    sender: &SyncSender<Result<Prepared, String>>,
    cancel: &AtomicBool,
    paused: &AtomicBool,
) -> Result<(), String> {
    let mut skip = engine.latency_frames() * 2;
    let mut pcm = Vec::new();
    loop {
        wait(cancel, paused)?;
        let frames = read(&mut pcm)?;
        let (vocals, accompaniment) = if frames == 0 {
            engine
                .drain()
                .map_err(|e| format!("AI tail failed: {e:?}"))?
        } else {
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
                .map_err(|e| format!("AI separation failed: {e:?}"))?;
            if report.consumed_frames != frames || report.produced_frames != frames {
                return Err("AI returned an incomplete file block".into());
            }
            (vocals, accompaniment)
        };
        let drop = skip.min(vocals.len());
        skip -= drop;
        if drop < vocals.len() {
            for (v, a) in vocals[drop..]
                .chunks(16384)
                .zip(accompaniment[drop..].chunks(16384))
            {
                send(
                    sender,
                    Ok(Prepared::Stems {
                        vocals: v.to_vec(),
                        accompaniment: a.to_vec(),
                    }),
                    cancel,
                    paused,
                )?;
            }
        }
        if frames == 0 {
            break;
        }
    }
    send(sender, Ok(Prepared::End), cancel, paused)
}

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;
