pub trait AudioProcessor: Send {
    fn process_stereo_interleaved(&mut self, samples: &mut [f32]);
    fn latency_frames(&self) -> usize;
}
