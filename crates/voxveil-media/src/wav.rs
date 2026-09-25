use super::{
    append_stereo_sample, finite_sample, validate_media_info, MediaError, MediaInfo, PcmDecoder,
    MAX_READ_FRAMES,
};
use hound::{SampleFormat, WavReader};
use std::fs::File;
use std::path::Path;

pub(super) struct WavDecoder {
    reader: WavReader<std::io::BufReader<File>>,
    info: MediaInfo,
    position: u64,
}

impl WavDecoder {
    pub(super) fn open(path: &Path) -> Result<Self, MediaError> {
        let reader = WavReader::open(path).map_err(|error| MediaError(error.to_string()))?;
        let spec = reader.spec();
        let info = validate_media_info(MediaInfo {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            total_frames: Some(reader.duration() as u64),
        })?;
        if spec.sample_format == SampleFormat::Float && spec.bits_per_sample != 32 {
            return Err(MediaError(
                "only 32-bit float WAV files are supported".into(),
            ));
        }
        if spec.sample_format == SampleFormat::Int
            && !matches!(spec.bits_per_sample, 8 | 16 | 24 | 32)
        {
            return Err(MediaError("unsupported PCM WAV bit depth".into()));
        }
        Ok(Self {
            reader,
            info,
            position: 0,
        })
    }
}

impl PcmDecoder for WavDecoder {
    fn info(&self) -> MediaInfo {
        self.info
    }
    fn position_frames(&self) -> u64 {
        self.position
    }

    fn read_frames(
        &mut self,
        max_frames: usize,
        output: &mut Vec<f32>,
    ) -> Result<usize, MediaError> {
        output.clear();
        let channels = self.info.channels as usize;
        let sample_count = max_frames.saturating_mul(channels);
        let scale = 2_f32.powi(self.reader.spec().bits_per_sample as i32 - 1);
        match self.reader.spec().sample_format {
            SampleFormat::Int => {
                let mut samples = self.reader.samples::<i32>();
                for sample in samples.by_ref().take(sample_count) {
                    let value =
                        sample.map_err(|error| MediaError(error.to_string()))? as f32 / scale;
                    if channels == 1 {
                        append_stereo_sample(output, value, None)?;
                    } else {
                        output.push(finite_sample(value)?);
                    }
                }
            }
            SampleFormat::Float => {
                let mut samples = self.reader.samples::<f32>();
                for sample in samples.by_ref().take(sample_count) {
                    let value =
                        finite_sample(sample.map_err(|error| MediaError(error.to_string()))?)?;
                    if channels == 1 {
                        append_stereo_sample(output, value, None)?;
                    } else {
                        output.push(value);
                    }
                }
            }
        }
        if channels == 2 && output.len() > MAX_READ_FRAMES * 2 {
            return Err(MediaError(
                "decoder output exceeded the bounded PCM chunk".into(),
            ));
        }
        if channels == 2 && output.len() % 2 != 0 {
            return Err(MediaError("WAV ended between stereo samples".into()));
        }
        let frames = if channels == 1 {
            output.len() / 2
        } else {
            output.len() / 2
        };
        self.position = self.position.saturating_add(frames as u64);
        Ok(frames)
    }

    fn seek(&mut self, frame: u64) -> Result<u64, MediaError> {
        let total = self.info.total_frames.unwrap_or(0);
        if frame > total || frame > u32::MAX as u64 {
            return Err(MediaError(
                "seek position is beyond the end of the file".into(),
            ));
        }
        self.reader
            .seek(frame as u32)
            .map_err(|error| MediaError(error.to_string()))?;
        self.position = frame;
        Ok(frame)
    }
}
