use crate::sample::process_capture_f32le_stereo;
use voxveil_ai::live::{LiveAi, LiveError};
use voxveil_dsp::SpectralCenterSuppressor;
use voxveil_types::{ClassicSuppressionProfile, SystemAiStatus, VocalLevel};

pub(crate) struct RelayProcessor {
    dsp: SpectralCenterSuppressor,
    ai: Option<LiveAi>,
    pub status: SystemAiStatus,
    level: u8,
    last: [f32; 2],
    fade: usize,
    log_path: Option<std::path::PathBuf>,
}
impl RelayProcessor {
    pub fn fill_idle(
        &mut self,
        idle: &mut crate::relay_idle::IdleCapture,
        now: std::time::Instant,
        queue: &mut crate::relay_buffer::RelayBuffer,
        padding: usize,
        drained: bool,
        buffer: &mut [u8],
    ) -> Result<usize, String> {
        let frames = idle.frames_due(now, drained, queue.idle_fill_frames(padding));
        if frames != 0 {
            let buffer = &mut buffer[..frames * crate::relay_buffer::FRAME_BYTES];
            self.process(buffer, true)?;
            queue.push_capture(buffer);
            queue.reset_drift();
        }
        Ok(frames)
    }
    pub fn runtime_state(&self) -> crate::relay_engine::RelayRuntimeState {
        if self.status == SystemAiStatus::Off {
            crate::relay_engine::RelayRuntimeState::Running
        } else {
            crate::relay_engine::RelayRuntimeState::Ai(self.status)
        }
    }
    pub fn new(
        rate: u32,
        level: u8,
        profile: ClassicSuppressionProfile,
        options: crate::RelayAiOptions,
    ) -> Self {
        let ai = options.config.map(|config| LiveAi::new(config, rate));
        let status = if ai.is_some() {
            SystemAiStatus::Buffering
        } else {
            SystemAiStatus::Off
        };
        Self {
            dsp: SpectralCenterSuppressor::new(
                rate,
                VocalLevel::new(level.min(100) as f32 / 100.0).unwrap(),
                profile,
            ),
            ai,
            status,
            level,
            last: [0.0; 2],
            fade: 0,
            log_path: options.log_path,
        }
    }
    pub fn set_level(&mut self, level: u8) {
        self.level = level.min(100);
        self.dsp
            .set_vocal_level(VocalLevel::new(self.level as f32 / 100.0).unwrap());
    }
    pub fn set_profile(&mut self, profile: ClassicSuppressionProfile) {
        self.dsp.set_profile(profile);
    }
    pub fn discontinuity(&mut self) {
        if self.ai.is_some() {
            self.fallback(
                "AI capture timeline became discontinuous",
                SystemAiStatus::FellBehind,
            );
        }
    }
    fn fallback(&mut self, reason: &str, status: SystemAiStatus) {
        crate::relay_ai_log::fallback(self.log_path.as_deref(), reason);
        self.ai.take();
        self.status = status;
        self.fade = 256;
    }
    pub fn process(&mut self, bytes: &mut [u8], silent: bool) -> Result<(), String> {
        let pcm: Vec<f32> = if self.ai.is_some() {
            bytes
                .chunks_exact(4)
                .map(|sample| {
                    if silent {
                        0.0
                    } else {
                        f32::from_le_bytes(sample.try_into().unwrap())
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        // Keep DSP warm so a missed AI deadline can transition without a second startup gap.
        process_capture_f32le_stereo(bytes, silent, &mut self.dsp).map_err(str::to_string)?;
        if self.ai.is_none() && self.fade == 0 {
            return Ok(());
        }
        if let Some(ai) = self.ai.as_mut() {
            match ai.process(&pcm, self.level) {
                Ok((mixed, buffering)) => {
                    self.status = if buffering {
                        SystemAiStatus::Buffering
                    } else {
                        SystemAiStatus::Running
                    };
                    for (target, sample) in bytes.chunks_exact_mut(4).zip(mixed) {
                        target.copy_from_slice(&sample.to_le_bytes());
                    }
                }
                Err(error) => {
                    let status = match error {
                        LiveError::Behind(_) => SystemAiStatus::FellBehind,
                        LiveError::Backend(_) => SystemAiStatus::Failed,
                    };
                    self.fallback(&error.to_string(), status);
                }
            }
        }
        for frame in bytes.chunks_exact_mut(8) {
            for ch in 0..2 {
                let target = &mut frame[ch * 4..ch * 4 + 4];
                let sample = f32::from_le_bytes(target.try_into().unwrap());
                let weight = self.fade as f32 / 256.0;
                let sample = sample * (1.0 - weight) + self.last[ch] * weight;
                target.copy_from_slice(&sample.to_le_bytes());
                if self.fade == 0 {
                    self.last[ch] = sample;
                }
            }
            self.fade = self.fade.saturating_sub(1);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "relay_idle_processing_tests.rs"]
mod tests;
