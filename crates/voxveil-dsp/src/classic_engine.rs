use std::f32::consts::FRAC_PI_2;

use voxveil_audio_core::AudioProcessor;
use voxveil_types::{ClassicSuppressionProfile, VocalLevel};

use crate::{SpectralCenterSuppressor, VocalSeparator};

const CROSSFADE_SECONDS: f32 = 0.020;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EngineKind {
    Legacy,
    Separator,
}

fn kind_for(profile: ClassicSuppressionProfile) -> EngineKind {
    if profile == ClassicSuppressionProfile::Isolation {
        EngineKind::Separator
    } else {
        EngineKind::Legacy
    }
}

#[derive(Clone, Copy, Debug)]
struct Transition {
    to: EngineKind,
    warm_remaining: usize,
    fade_position: usize,
}

/// Owns the legacy center suppressor and the vocal separator. Legacy-to-legacy profile
/// changes stay hot; switching to or from Isolation warms the incoming engine for twice
/// its latency, then equal-power crossfades over 20 ms. No allocation after `new`.
pub struct ClassicEngine {
    sample_rate: u32,
    vocal_level: VocalLevel,
    profile: ClassicSuppressionProfile,
    legacy_profile: ClassicSuppressionProfile,
    legacy: SpectralCenterSuppressor,
    separator: VocalSeparator,
    active: EngineKind,
    transition: Option<Transition>,
    fade_frames: usize,
}

impl ClassicEngine {
    pub fn new(
        sample_rate: u32,
        vocal_level: VocalLevel,
        profile: ClassicSuppressionProfile,
    ) -> Self {
        let legacy_profile = if kind_for(profile) == EngineKind::Legacy {
            profile
        } else {
            ClassicSuppressionProfile::default()
        };
        Self {
            sample_rate,
            vocal_level,
            profile,
            legacy_profile,
            legacy: SpectralCenterSuppressor::new(sample_rate, vocal_level, legacy_profile),
            separator: VocalSeparator::new(sample_rate, vocal_level),
            active: kind_for(profile),
            transition: None,
            fade_frames: ((sample_rate.max(8_000) as f32 * CROSSFADE_SECONDS) as usize).max(1),
        }
    }

    pub fn profile(&self) -> ClassicSuppressionProfile {
        self.profile
    }

    pub fn set_vocal_level(&mut self, vocal_level: VocalLevel) {
        self.vocal_level = vocal_level;
        self.legacy.set_vocal_level(vocal_level);
        self.separator.set_vocal_level(vocal_level);
    }

    pub fn set_profile(&mut self, profile: ClassicSuppressionProfile) {
        if profile == self.profile {
            return;
        }
        let from = kind_for(self.profile);
        let to = kind_for(profile);
        self.profile = profile;
        if to == EngineKind::Legacy {
            self.legacy_profile = profile;
        }
        match (from, to) {
            (EngineKind::Legacy, EngineKind::Legacy) => self.legacy.set_profile(profile),
            (EngineKind::Separator, EngineKind::Separator) => {}
            _ => self.begin_transition(to),
        }
    }

    fn begin_transition(&mut self, to: EngineKind) {
        if to == self.active {
            // A switch was still warming up: cancel it and keep the audible engine.
            self.transition = None;
            if to == EngineKind::Legacy {
                self.legacy.set_profile(self.legacy_profile);
            }
            return;
        }
        match to {
            EngineKind::Legacy => {
                self.legacy = SpectralCenterSuppressor::new(
                    self.sample_rate,
                    self.vocal_level,
                    self.legacy_profile,
                );
            }
            EngineKind::Separator => self.separator.reset(),
        }
        // Past its latency the incoming engine still ramps in over one more window; wait
        // for steady-state output before fading.
        let warm_remaining = 2 * self.latency_of(to) + 1;
        self.transition = Some(Transition { to, warm_remaining, fade_position: 0 });
    }

    fn latency_of(&self, kind: EngineKind) -> usize {
        match kind {
            EngineKind::Legacy => self.legacy.latency_frames(),
            EngineKind::Separator => self.separator.latency_frames(),
        }
    }

    fn process_with(&mut self, kind: EngineKind, samples: &mut [f32]) {
        match kind {
            EngineKind::Legacy => self.legacy.process_stereo_interleaved(samples),
            EngineKind::Separator => self.separator.process_stereo_interleaved(samples),
        }
    }
}

impl AudioProcessor for ClassicEngine {
    fn process_stereo_interleaved(&mut self, samples: &mut [f32]) {
        if self.transition.is_none() {
            self.process_with(self.active, samples);
            return;
        }
        for frame in samples.chunks_exact_mut(2) {
            let Some(mut transition) = self.transition else {
                self.process_with(self.active, frame);
                continue;
            };
            let mut outgoing = [frame[0], frame[1]];
            let mut incoming = outgoing;
            self.process_with(self.active, &mut outgoing);
            self.process_with(transition.to, &mut incoming);
            if transition.warm_remaining > 0 {
                transition.warm_remaining -= 1;
                frame.copy_from_slice(&outgoing);
            } else {
                let x = (transition.fade_position as f32 + 0.5) / self.fade_frames as f32;
                let (fade_out, fade_in) = ((x * FRAC_PI_2).cos(), (x * FRAC_PI_2).sin());
                frame[0] = outgoing[0] * fade_out + incoming[0] * fade_in;
                frame[1] = outgoing[1] * fade_out + incoming[1] * fade_in;
                transition.fade_position += 1;
            }
            if transition.fade_position >= self.fade_frames {
                self.active = transition.to;
                self.transition = None;
            } else {
                self.transition = Some(transition);
            }
        }
    }

    fn latency_frames(&self) -> usize {
        self.latency_of(self.active)
    }
}
