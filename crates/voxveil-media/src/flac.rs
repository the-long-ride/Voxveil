use super::{
    append_stereo_sample, validate_media_info, MediaError, MediaInfo, PcmDecoder, MAX_READ_FRAMES,
};
use claxon::FlacReader;
use std::fs::File;
use std::path::{Path, PathBuf};

pub(super) struct FlacDecoder {
    path: PathBuf,
    reader: FlacReader<File>,
    info: MediaInfo,
    bits_per_sample: u32,
    position: u64,
    pending: Vec<f32>,
    pending_offset: usize,
}

impl FlacDecoder {
    pub(super) fn open(path: &Path) -> Result<Self, MediaError> {
        let reader = FlacReader::open(path).map_err(|error| MediaError(error.to_string()))?;
        let stream = reader.streaminfo();
        let info = validate_media_info(MediaInfo {
            sample_rate: stream.sample_rate,
            channels: stream.channels as u16,
            total_frames: stream.samples,
        })?;
        if !(8..=32).contains(&stream.bits_per_sample) {
            return Err(MediaError("unsupported FLAC bit depth".into()));
        }
        Ok(Self {
            path: path.to_path_buf(),
            reader,
            info,
            bits_per_sample: stream.bits_per_sample,
            position: 0,
            pending: Vec::new(),
            pending_offset: 0,
        })
    }

    fn decode_block(&mut self) -> Result<bool, MediaError> {
        let block = self
            .reader
            .blocks()
            .read_next_or_eof(Vec::new())
            .map_err(|error| MediaError(error.to_string()))?;
        let Some(block) = block else { return Ok(false) };
        if block.channels() as u16 != self.info.channels {
            return Err(MediaError(
                "FLAC channel layout changed during playback".into(),
            ));
        }
        if block.duration() > u16::MAX as u32 {
            return Err(MediaError(
                "FLAC block exceeded the bounded decoder limit".into(),
            ));
        }
        self.pending.clear();
        self.pending_offset = 0;
        let scale = 2_f32.powi(self.bits_per_sample as i32 - 1);
        for frame in 0..block.duration() {
            let left = block.sample(0, frame) as f32 / scale;
            let right = if self.info.channels == 2 {
                Some(block.sample(1, frame) as f32 / scale)
            } else {
                None
            };
            append_stereo_sample(&mut self.pending, left, right)?;
        }
        if self.pending.len() > (u16::MAX as usize) * 2 {
            return Err(MediaError(
                "FLAC block exceeded the bounded decoder limit".into(),
            ));
        }
        Ok(!self.pending.is_empty())
    }
}

impl PcmDecoder for FlacDecoder {
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
        let target_samples = max_frames * 2;
        while output.len() < target_samples {
            if self.pending_offset >= self.pending.len() && !self.decode_block()? {
                break;
            }
            let remaining = target_samples - output.len();
            let available = self.pending.len() - self.pending_offset;
            let take = remaining.min(available);
            output
                .extend_from_slice(&self.pending[self.pending_offset..self.pending_offset + take]);
            self.pending_offset += take;
        }
        let frames = output.len() / 2;
        self.position = self.position.saturating_add(frames as u64);
        Ok(frames)
    }

    fn seek(&mut self, frame: u64) -> Result<u64, MediaError> {
        if self.info.total_frames.is_some_and(|total| frame > total) {
            return Err(MediaError(
                "seek position is beyond the end of the file".into(),
            ));
        }
        self.reader =
            FlacReader::open(&self.path).map_err(|error| MediaError(error.to_string()))?;
        self.position = 0;
        self.pending.clear();
        self.pending_offset = 0;
        let mut discard = Vec::new();
        while self.position < frame {
            let count = ((frame - self.position) as usize).min(MAX_READ_FRAMES);
            let read = self.read_frames(count.max(1), &mut discard)?;
            if read == 0 {
                break;
            }
        }
        Ok(self.position)
    }
}
