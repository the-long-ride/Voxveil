use voxveil_audio_core::AudioProcessor;

pub fn process_f32le_stereo(
    bytes: &mut [u8],
    processor: &mut dyn AudioProcessor,
) -> Result<(), &'static str> {
    if !bytes.len().is_multiple_of(8) {
        return Err("audio buffer must contain complete stereo f32 frames");
    }

    for frame in bytes.chunks_exact_mut(8) {
        let left = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        let right = f32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);
        let mut samples = [left, right];
        processor.process_stereo_interleaved(&mut samples);
        frame[0..4].copy_from_slice(&samples[0].to_le_bytes());
        frame[4..8].copy_from_slice(&samples[1].to_le_bytes());
    }
    Ok(())
}

pub(crate) fn process_capture_f32le_stereo(
    bytes: &mut [u8],
    silent: bool,
    processor: &mut dyn AudioProcessor,
) -> Result<(), &'static str> {
    if silent {
        bytes.fill(0);
    }
    process_f32le_stereo(bytes, processor)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StatefulGain {
        calls: usize,
        gain: f32,
    }

    impl AudioProcessor for StatefulGain {
        fn process_stereo_interleaved(&mut self, samples: &mut [f32]) {
            self.calls += 1;
            for sample in samples {
                *sample *= self.gain;
            }
        }

        fn latency_frames(&self) -> usize {
            0
        }
    }

    fn stereo_bytes(left: f32, right: f32) -> Vec<u8> {
        [left.to_le_bytes(), right.to_le_bytes()].concat()
    }

    #[test]
    fn supplied_processor_is_retained_across_packets() {
        let mut processor = StatefulGain { calls: 0, gain: 0.5 };
        let mut first = stereo_bytes(0.8, -0.4);
        let mut second = stereo_bytes(0.2, 0.6);

        process_f32le_stereo(&mut first, &mut processor).unwrap();
        process_f32le_stereo(&mut second, &mut processor).unwrap();

        assert_eq!(processor.calls, 2);
        assert!((f32::from_le_bytes(first[0..4].try_into().unwrap()) - 0.4).abs() < 1.0e-6);
        assert!((f32::from_le_bytes(second[4..8].try_into().unwrap()) - 0.3).abs() < 1.0e-6);
    }

    #[test]
    fn silent_capture_packet_still_advances_processor_state() {
        let mut processor = StatefulGain { calls: 0, gain: 0.5 };
        let mut packet = stereo_bytes(0.8, -0.4);

        process_capture_f32le_stereo(&mut packet, true, &mut processor).unwrap();

        assert_eq!(processor.calls, 1);
        assert!(packet.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn rejects_partial_frames() {
        let mut processor = StatefulGain { calls: 0, gain: 1.0 };
        assert!(process_f32le_stereo(&mut [0; 7], &mut processor).is_err());
        assert_eq!(processor.calls, 0);
    }
}
