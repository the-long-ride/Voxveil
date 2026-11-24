use std::collections::VecDeque;
use voxveil_model_api::SeparationError;

pub(crate) trait ChunkModel: Send {
    fn run(&mut self, stereo: &[f32]) -> Result<Vec<f32>, SeparationError>;
}

pub(crate) struct Chunks {
    pub frames: usize,
    overlap: usize,
    input: VecDeque<f32>,
    tail: Option<Vec<f32>>,
    pub output: VecDeque<[f32; 2]>,
    first: bool,
}

impl Chunks {
    pub fn new(frames: usize, overlap: usize) -> Self {
        Self {
            frames,
            overlap,
            input: std::iter::repeat_n(0.0, overlap * 2).collect(),
            tail: None,
            output: std::iter::repeat_n([0.0; 2], frames * 2).collect(),
            first: true,
        }
    }

    pub fn push(
        &mut self,
        input: &[f32],
        model: &mut dyn ChunkModel,
    ) -> Result<(), SeparationError> {
        self.input.extend(input);
        let step = self.frames - self.overlap;
        while self.input.len() >= self.frames * 2 {
            let block: Vec<_> = self.input.iter().take(self.frames * 2).copied().collect();
            let mut accompaniment = model.run(&block)?;
            if accompaniment.len() != block.len() || accompaniment.iter().any(|x| !x.is_finite()) {
                return Err(SeparationError::BackendFailure);
            }
            if let Some(tail) = &self.tail {
                for i in 0..self.overlap {
                    let weight =
                        (1.0 - (std::f32::consts::PI * i as f32 / self.overlap as f32).cos()) * 0.5;
                    for ch in 0..2 {
                        let j = i * 2 + ch;
                        accompaniment[j] = tail[j] * (1.0 - weight) + accompaniment[j] * weight;
                    }
                }
            }
            let start = if self.first { self.overlap } else { 0 };
            for i in start * 2..step * 2 {
                self.output
                    .push_back([block[i] - accompaniment[i], accompaniment[i]]);
            }
            self.first = false;
            self.tail = Some(accompaniment[step * 2..].to_vec());
            self.input.drain(..step * 2);
        }
        Ok(())
    }
}
