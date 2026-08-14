#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeparationCapabilities {
    pub stereo: bool,
    pub streaming: bool,
    pub commercial_redistribution_cleared: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeparationReport {
    pub consumed_frames: usize,
    pub produced_frames: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeparationError {
    ModelUnavailable,
    InvalidBuffer,
    BackendFailure,
}

pub struct StemOutput<'a> {
    pub vocals: &'a mut [f32],
    pub accompaniment: &'a mut [f32],
}

pub trait SeparationEngine: Send {
    fn process(
        &mut self,
        interleaved_stereo: &[f32],
        output: &mut StemOutput<'_>,
    ) -> Result<SeparationReport, SeparationError>;

    fn latency_frames(&self) -> usize;
    fn capabilities(&self) -> SeparationCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_contract_keeps_license_clearance_explicit() {
        let capabilities = SeparationCapabilities {
            stereo: true,
            streaming: true,
            commercial_redistribution_cleared: false,
        };
        assert!(!capabilities.commercial_redistribution_cleared);
    }
}
