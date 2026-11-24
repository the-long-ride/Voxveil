use super::{FileEngine, Prepared, send, wait};
use std::sync::{atomic::AtomicBool, mpsc::SyncSender};
use voxveil_model_api::StemOutput;

pub(super) trait Sink {
    fn work_start(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn work_done(&mut self, _frames: usize) -> Result<(), String> {
        Ok(())
    }
    fn emit(&mut self, message: Prepared) -> Result<(), String>;
}
struct Channel<'a> {
    sender: &'a SyncSender<Result<Prepared, String>>,
    cancel: &'a AtomicBool,
    paused: &'a AtomicBool,
}
impl Sink for Channel<'_> {
    fn emit(&mut self, message: Prepared) -> Result<(), String> {
        send(self.sender, Ok(message), self.cancel, self.paused)
    }
}
pub(super) fn produce(
    engine: &mut dyn FileEngine,
    read: impl FnMut(&mut Vec<f32>) -> Result<usize, String>,
    sender: &SyncSender<Result<Prepared, String>>,
    cancel: &AtomicBool,
    paused: &AtomicBool,
) -> Result<(), String> {
    produce_into(
        engine,
        read,
        &mut Channel {
            sender,
            cancel,
            paused,
        },
        cancel,
        paused,
    )
}
pub(super) fn produce_into(
    engine: &mut dyn FileEngine,
    mut read: impl FnMut(&mut Vec<f32>) -> Result<usize, String>,
    sink: &mut dyn Sink,
    cancel: &AtomicBool,
    paused: &AtomicBool,
) -> Result<(), String> {
    let mut skip = engine.latency_frames() * 2;
    let mut pcm = Vec::new();
    loop {
        wait(cancel, paused)?;
        sink.work_start()?;
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
        if vocals.len() != accompaniment.len() || vocals.len() % 2 != 0 {
            return Err("AI returned invalid stereo stems".into());
        }
        let drop = skip.min(vocals.len());
        skip -= drop;
        if drop < vocals.len() {
            for (v, a) in vocals[drop..]
                .chunks(16384)
                .zip(accompaniment[drop..].chunks(16384))
            {
                if cancel.load(std::sync::atomic::Ordering::Acquire) {
                    return Err("AI preparation cancelled".into());
                }
                sink.emit(Prepared::Stems {
                    vocals: v.to_vec(),
                    accompaniment: a.to_vec(),
                })?;
            }
        }
        sink.work_done(frames)?;
        if frames == 0 {
            break;
        }
    }
    sink.emit(Prepared::End)
}
