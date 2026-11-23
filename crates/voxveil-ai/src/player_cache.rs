use super::{Prepared, prefetch::Prefetch, preparation::Sink, temporary::anonymous_file};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(super) struct Cache {
    file: Option<File>,
    policy: Option<Prefetch>,
    written: u64,
    committed: u64,
    read: u64,
    finished: bool,
    started: bool,
    end_sent: bool,
    clock: Box<dyn Fn() -> Duration + Send>,
}
impl Cache {
    pub fn new() -> Self {
        let start = Instant::now();
        Self::with_clock(Box::new(move || start.elapsed()))
    }
    pub(super) fn with_clock(clock: Box<dyn Fn() -> Duration + Send>) -> Self {
        Self {
            file: None,
            policy: None,
            written: 0,
            committed: 0,
            read: 0,
            finished: false,
            started: false,
            end_sent: false,
            clock,
        }
    }
    pub fn configure(&mut self, rate: u32, total: Option<u64>, context: usize) {
        self.policy = Some(Prefetch::new(rate, total, context));
    }
    pub fn wait_ms(&self) -> Option<u64> {
        if self.started {
            return None;
        }
        self.policy
            .as_ref()?
            .wait_ms((self.clock)(), self.committed, self.finished)
    }
    fn push(&mut self, message: Prepared) -> Result<(), String> {
        match message {
            Prepared::Stems {
                vocals,
                accompaniment,
            } => {
                if self.file.is_none() {
                    self.file = Some(anonymous_file()?);
                }
                let bytes: Vec<u8> = vocals
                    .chunks_exact(2)
                    .zip(accompaniment.chunks_exact(2))
                    .flat_map(|(v, a)| [v[0], v[1], a[0], a[1]])
                    .flat_map(f32::to_le_bytes)
                    .collect();
                let file = self.file.as_mut().unwrap();
                file.seek(SeekFrom::Start(
                    self.written
                        .checked_mul(16)
                        .ok_or("AI stem cache is too large")?,
                ))
                .map_err(|_| "AI stem cache seek failed")?;
                file.write_all(&bytes)
                    .map_err(|_| "AI stem cache write failed; check free disk space")?;
                self.written += vocals.len() as u64 / 2;
            }
            Prepared::End => self.finished = true,
            Prepared::Position(_) => {
                return Err("AI stem cache received an unexpected position".into());
            }
        }
        Ok(())
    }
    pub fn poll(&mut self) -> Result<Option<Prepared>, String> {
        if !self.started {
            if self.committed == 0 && !self.finished {
                return Ok(None);
            }
            if self.wait_ms() != Some(0) {
                return Ok(None);
            }
            self.started = true;
        }
        let frames = (self.committed - self.read).min(8192) as usize;
        if frames == 0 {
            if self.finished && !self.end_sent {
                self.end_sent = true;
                return Ok(Some(Prepared::End));
            }
            return Ok(None);
        }
        let file = self.file.as_mut().ok_or("AI stem cache is missing")?;
        file.seek(SeekFrom::Start(self.read * 16))
            .map_err(|_| "AI stem cache seek failed")?;
        let mut bytes = vec![0; frames * 16];
        file.read_exact(&mut bytes)
            .map_err(|_| "AI stem cache read failed")?;
        let mut vocals = Vec::with_capacity(frames * 2);
        let mut accompaniment = Vec::with_capacity(frames * 2);
        for frame in bytes.chunks_exact(16) {
            for ch in 0..2 {
                vocals.push(f32::from_le_bytes(
                    frame[ch * 4..ch * 4 + 4].try_into().unwrap(),
                ));
                accompaniment.push(f32::from_le_bytes(
                    frame[8 + ch * 4..12 + ch * 4].try_into().unwrap(),
                ));
            }
        }
        self.read += frames as u64;
        Ok(Some(Prepared::Stems {
            vocals,
            accompaniment,
        }))
    }
}
pub(super) struct Writer(pub Arc<Mutex<Cache>>);
impl Sink for Writer {
    fn work_start(&mut self) -> Result<(), String> {
        let mut cache = self.0.lock().map_err(|_| "AI cache lock is poisoned")?;
        let now = (cache.clock)();
        cache
            .policy
            .as_mut()
            .ok_or("AI cache not configured")?
            .start(now);
        Ok(())
    }
    fn work_done(&mut self, frames: usize) -> Result<(), String> {
        let mut cache = self.0.lock().map_err(|_| "AI cache lock is poisoned")?;
        let now = (cache.clock)();
        cache
            .policy
            .as_mut()
            .ok_or("AI cache not configured")?
            .finish(now, frames);
        cache.committed = cache.written;
        if frames == 0 {
            cache.finished = true;
        }
        Ok(())
    }
    fn emit(&mut self, message: Prepared) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|_| "AI cache lock is poisoned")?
            .push(message)
    }
}

#[cfg(test)]
#[path = "player_prefetch_tests.rs"]
mod tests;
