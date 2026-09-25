use super::mp3_scan::{scan_mp3, Mp3SeekPoint};
use super::{normalize_interleaved, MediaError, MediaInfo, PcmDecoder, MAX_READ_FRAMES};
use nanomp3::{Decoder as Mp3CoreDecoder, MAX_SAMPLES_PER_FRAME};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const MP3_INPUT_WINDOW: usize = 16 * 1024;

pub(super) struct Mp3Decoder {
    file: File,
    decoder: Mp3CoreDecoder,
    info: MediaInfo,
    points: Vec<Mp3SeekPoint>,
    input: Box<[u8; MP3_INPUT_WINDOW]>,
    input_len: usize,
    eof: bool,
    decoded: [f32; MAX_SAMPLES_PER_FRAME],
    pending: Vec<f32>,
    pending_offset: usize,
    position: u64,
}

impl Mp3Decoder {
    pub(super) fn open(path: &Path) -> Result<Self, MediaError> {
        let scan = scan_mp3(path)?;
        let mut decoder = Self {
            file: File::open(path)?,
            decoder: Mp3CoreDecoder::new(),
            info: scan.info,
            points: scan.points,
            input: Box::new([0; MP3_INPUT_WINDOW]),
            input_len: 0,
            eof: false,
            decoded: [0.0; MAX_SAMPLES_PER_FRAME],
            pending: Vec::new(),
            pending_offset: 0,
            position: 0,
        };
        decoder.prime()?;
        Ok(decoder)
    }

    fn refill(&mut self) -> Result<(), MediaError> {
        if self.input_len == self.input.len() || self.eof {
            return Ok(());
        }
        let read = self.file.read(&mut self.input[self.input_len..])?;
        if read == 0 {
            self.eof = true;
        } else {
            self.input_len += read;
        }
        Ok(())
    }

    fn next_frame(&mut self) -> Result<Option<(usize, nanomp3::FrameInfo)>, MediaError> {
        loop {
            self.refill()?;
            if self.input_len == 0 && self.eof {
                return Ok(None);
            }
            let (consumed, frame) = self
                .decoder
                .decode(&self.input[..self.input_len], &mut self.decoded);
            if consumed > self.input_len {
                return Err(MediaError(
                    "MP3 decoder returned an invalid input offset".into(),
                ));
            }
            if consumed > 0 {
                self.input.copy_within(consumed..self.input_len, 0);
                self.input_len -= consumed;
            }
            if let Some(frame) = frame {
                if consumed == 0 {
                    return Err(MediaError("MP3 decoder made no input progress".into()));
                }
                let channels = frame.channels.num() as u16;
                if frame.sample_rate != self.info.sample_rate || channels != self.info.channels {
                    return Err(MediaError(
                        "MP3 stream changed sample rate or channel layout".into(),
                    ));
                }
                let sample_count = frame
                    .samples_produced
                    .checked_mul(channels as usize)
                    .ok_or_else(|| MediaError("MP3 frame size overflow".into()))?;
                if sample_count > self.decoded.len() {
                    return Err(MediaError("MP3 decoder exceeded its output buffer".into()));
                }
                return Ok(Some((sample_count, frame)));
            }
            if consumed == 0 && self.input_len == self.input.len() {
                return Err(MediaError("invalid or unsupported MP3 frame".into()));
            }
            if consumed == 0 && self.eof {
                return Err(MediaError("MP3 file ended in a truncated frame".into()));
            }
        }
    }

    fn prime(&mut self) -> Result<(), MediaError> {
        let (samples, frame) = self
            .next_frame()?
            .ok_or_else(|| MediaError("MP3 file contains no decodable frames".into()))?;
        let frames = frame.samples_produced;
        self.pending.clear();
        normalize_interleaved(
            &self.decoded[..samples],
            frame.channels.num() as u16,
            &mut self.pending,
        )?;
        if self.pending.len() != frames * 2 {
            return Err(MediaError(
                "MP3 decoder returned an incomplete channel frame".into(),
            ));
        }
        self.pending_offset = 0;
        Ok(())
    }

    fn reset_at(&mut self, point: Mp3SeekPoint) -> Result<(), MediaError> {
        self.file.seek(SeekFrom::Start(point.offset))?;
        self.decoder = Mp3CoreDecoder::new();
        self.input_len = 0;
        self.eof = false;
        self.pending.clear();
        self.pending_offset = 0;
        self.position = point.frame;
        self.prime()?;
        Ok(())
    }
}

impl PcmDecoder for Mp3Decoder {
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
        let target = max_frames * 2;
        while output.len() < target {
            if self.pending_offset >= self.pending.len() {
                let Some((samples, frame)) = self.next_frame()? else {
                    break;
                };
                self.pending.clear();
                normalize_interleaved(
                    &self.decoded[..samples],
                    frame.channels.num() as u16,
                    &mut self.pending,
                )?;
                self.pending_offset = 0;
            }
            let take = (target - output.len()).min(self.pending.len() - self.pending_offset);
            output
                .extend_from_slice(&self.pending[self.pending_offset..self.pending_offset + take]);
            self.pending_offset += take;
        }
        let frames = output.len() / 2;
        self.position = self.position.saturating_add(frames as u64);
        Ok(frames)
    }

    fn seek(&mut self, frame: u64) -> Result<u64, MediaError> {
        let total = self.info.total_frames.unwrap_or(0);
        if frame > total {
            return Err(MediaError(
                "seek position is beyond the end of the file".into(),
            ));
        }
        let preroll = (self.info.sample_rate as u64).saturating_mul(2);
        let start = frame.saturating_sub(preroll);
        let point = self
            .points
            .iter()
            .rev()
            .find(|point| point.frame <= start)
            .copied()
            .unwrap_or(Mp3SeekPoint {
                frame: 0,
                offset: 0,
            });
        self.reset_at(point)?;
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
