//! Buffered CPU separation. Call on a worker, never on an audio callback.
//! Model files are local, verified downloads; this crate has no network access.
#![forbid(unsafe_code)]

mod chunks;
pub mod player;
pub mod live;
mod resample;
mod runtime;
mod spectral;
#[cfg(test)]
mod tests;

use chunks::{ChunkModel, Chunks};
use std::path::Path;
use voxveil_model_api::{
    SeparationCapabilities, SeparationEngine, SeparationError, SeparationReport, StemOutput,
};

pub fn runtime_available(library: &Path) -> bool {
    library.is_absolute()
        && library.is_file()
        && ort::init_from(library)
            .map(|environment| {
                environment.commit();
                true
            })
            .unwrap_or(false)
        && ort::session::Session::builder().is_ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Spleeter,
    MdxInstHq3,
}

impl Model {
    pub fn native_chunk_frames(self) -> usize {
        match self {
            Self::Spleeter => 511 * 1024 + 4096,
            Self::MdxInstHq3 => 255 * 1024,
        }
    }

    pub fn latency_frames(self, sample_rate: u32) -> usize {
        (self.native_chunk_frames() as u64 * sample_rate as u64).div_ceil(44100) as usize
    }
}

struct HostModel {
    inner: runtime::OnnxModel,
    rate: usize,
    native_frames: usize,
}

impl ChunkModel for HostModel {
    fn run(&mut self, input: &[f32]) -> Result<Vec<f32>, SeparationError> {
        let native = resample::convert(input, self.rate, 44100, self.native_frames)?;
        let accompaniment = self.inner.separate(&native)?;
        resample::convert(&accompaniment, 44100, self.rate, input.len() / 2)
    }
}

pub struct AiEngine {
    model: Box<dyn ChunkModel>,
    chunks: Chunks,
    closed: bool,
    failed: bool,
}

impl AiEngine {
    /// `runtime_library` must name the app-owned ONNX Runtime library, not a PATH lookup.
    /// On Windows package `onnxruntime.dll` beside the executable. Initialize all engines
    /// with the same library path; ONNX Runtime initialization is process-global.
    pub fn load(
        model: Model,
        sample_rate: u32,
        directory: &Path,
        runtime_library: &Path,
    ) -> Result<Self, SeparationError> {
        if !matches!(sample_rate, 44100 | 48000) {
            return Err(SeparationError::InvalidBuffer);
        }
        let inner = runtime::OnnxModel::load(model, directory, runtime_library)?;
        Ok(Self::with_model(
            model,
            sample_rate,
            Box::new(HostModel {
                inner,
                rate: sample_rate as usize,
                native_frames: model.native_chunk_frames(),
            }),
        ))
    }

    fn with_model(model: Model, sample_rate: u32, backend: Box<dyn ChunkModel>) -> Self {
        let frames = model.latency_frames(sample_rate);
        let overlap = (16384_u64 * sample_rate as u64).div_ceil(44100) as usize;
        Self {
            model: backend,
            chunks: Chunks::new(frames, overlap),
            closed: false,
            failed: false,
        }
    }

    fn push(&mut self, input: &[f32]) -> Result<(), SeparationError> {
        if self.failed || self.closed {
            return Err(SeparationError::BackendFailure);
        }
        if let Err(error) = self.chunks.push(input, self.model.as_mut()) {
            self.failed = true;
            return Err(error);
        }
        Ok(())
    }

    /// Drain exactly the reported latency after the last input. Concatenating this
    /// with `process` output and removing the initial latency preserves track length.
    pub fn finish(&mut self) -> Result<(Vec<f32>, Vec<f32>), SeparationError> {
        while self.chunks.output.len() < self.latency_frames() * 2 {
            self.push(&vec![0.0; self.chunks.frames * 2])?;
        }
        if self.closed || self.failed {
            return Err(SeparationError::BackendFailure);
        }
        self.closed = true;
        let mut vocals = Vec::with_capacity(self.latency_frames() * 2);
        let mut accompaniment = Vec::with_capacity(self.latency_frames() * 2);
        for _ in 0..self.latency_frames() * 2 {
            let [v, a] = self
                .chunks
                .output
                .pop_front()
                .ok_or(SeparationError::BackendFailure)?;
            vocals.push(v);
            accompaniment.push(a);
        }
        Ok((vocals, accompaniment))
    }
}

impl SeparationEngine for AiEngine {
    fn process(
        &mut self,
        input: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError> {
        if input.len() % 2 != 0
            || output.vocals.len() < input.len()
            || output.accompaniment.len() < input.len()
            || input.iter().any(|x| !x.is_finite())
        {
            return Err(SeparationError::InvalidBuffer);
        }
        self.push(input)?;
        for i in 0..input.len() {
            let [v, a] = self
                .chunks
                .output
                .pop_front()
                .ok_or(SeparationError::BackendFailure)?;
            output.vocals[i] = v;
            output.accompaniment[i] = a;
        }
        Ok(SeparationReport {
            consumed_frames: input.len() / 2,
            produced_frames: input.len() / 2,
        })
    }

    fn latency_frames(&self) -> usize {
        self.chunks.frames
    }

    fn capabilities(&self) -> SeparationCapabilities {
        SeparationCapabilities {
            stereo: true,
            streaming: true,
            commercial_redistribution_cleared: true,
        }
    }
}
