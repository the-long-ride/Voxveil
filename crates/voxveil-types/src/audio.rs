use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleRate(u32);

impl SampleRate {
    pub fn new(hz: u32) -> Result<Self, &'static str> {
        if (8_000..=384_000).contains(&hz) {
            Ok(Self(hz))
        } else {
            Err("sample rate must be between 8 kHz and 384 kHz")
        }
    }

    pub const fn hz(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelCount(u8);

impl ChannelCount {
    pub fn new(channels: u8) -> Result<Self, &'static str> {
        if (1..=32).contains(&channels) {
            Ok(Self(channels))
        } else {
            Err("channel count must be between 1 and 32")
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_audio_format_values() {
        assert!(SampleRate::new(48_000).is_ok());
        assert!(SampleRate::new(1_000).is_err());
        assert!(ChannelCount::new(2).is_ok());
        assert!(ChannelCount::new(0).is_err());
    }
}
