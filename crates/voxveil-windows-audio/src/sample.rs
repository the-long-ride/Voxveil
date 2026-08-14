use voxveil_audio_core::AudioProcessor;
use voxveil_dsp::MidSideSuppressor;
use voxveil_types::VocalLevel;

pub fn process_f32le_stereo(bytes: &mut [u8], vocal_percent: u8) -> Result<(), &'static str> {
    if vocal_percent > 100 {
        return Err("vocal percent must be between 0 and 100");
    }
    if !bytes.len().is_multiple_of(8) {
        return Err("audio buffer must contain complete stereo f32 frames");
    }

    let level = VocalLevel::new(vocal_percent as f32 / 100.0)?;
    let mut suppressor = MidSideSuppressor::new(level);
    for frame in bytes.chunks_exact_mut(8) {
        let left = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        let right = f32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);
        let mut samples = [left, right];
        suppressor.process_stereo_interleaved(&mut samples);
        frame[0..4].copy_from_slice(&samples[0].to_le_bytes());
        frame[4..8].copy_from_slice(&samples[1].to_le_bytes());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stereo_bytes(left: f32, right: f32) -> Vec<u8> {
        [left.to_le_bytes(), right.to_le_bytes()].concat()
    }

    #[test]
    fn zero_percent_removes_centered_signal() {
        let mut bytes = stereo_bytes(0.75, 0.75);
        process_f32le_stereo(&mut bytes, 0).unwrap();
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0.0);
        assert_eq!(f32::from_le_bytes(bytes[4..8].try_into().unwrap()), 0.0);
    }

    #[test]
    fn full_percent_preserves_signal() {
        let original = stereo_bytes(0.25, -0.5);
        let mut bytes = original.clone();
        process_f32le_stereo(&mut bytes, 100).unwrap();
        assert_eq!(bytes, original);
    }

    #[test]
    fn rejects_partial_frames() {
        assert!(process_f32le_stereo(&mut [0; 7], 50).is_err());
    }
}
