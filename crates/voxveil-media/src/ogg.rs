use super::{
    append_stereo_sample, validate_media_info, MediaError, MediaInfo, PcmDecoder, MAX_READ_FRAMES,
};
use lewton::inside_ogg::OggStreamReader;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) struct OggDecoder {
    reader: OggStreamReader<std::io::BufReader<File>>,
    info: MediaInfo,
    serial: u32,
    position: u64,
    pending: Vec<f32>,
    pending_offset: usize,
}

impl OggDecoder {
    pub(super) fn open(path: &Path) -> Result<Self, MediaError> {
        let reader = OggStreamReader::new(std::io::BufReader::new(File::open(path)?))
            .map_err(|error| MediaError(error.to_string()))?;
        let serial = reader.stream_serial();
        let total_frames = scan_ogg_duration(path, serial)?;
        let info = validate_media_info(MediaInfo {
            sample_rate: reader.ident_hdr.audio_sample_rate,
            channels: reader.ident_hdr.audio_channels as u16,
            total_frames: Some(total_frames),
        })?;
        if reader.stream_serial() != serial {
            return Err(MediaError(
                "Ogg stream changed while it was being opened".into(),
            ));
        }
        Ok(Self {
            reader,
            info,
            serial,
            position: 0,
            pending: Vec::new(),
            pending_offset: 0,
        })
    }

    fn read_packet(&mut self) -> Result<Option<Vec<f32>>, MediaError> {
        let packet = self
            .reader
            .read_dec_packet_itl()
            .map_err(|error| MediaError(error.to_string()))?;
        let Some(packet) = packet else {
            return Ok(None);
        };
        if self.reader.stream_serial() != self.serial
            || self.reader.ident_hdr.audio_sample_rate != self.info.sample_rate
            || self.reader.ident_hdr.audio_channels as u16 != self.info.channels
        {
            return Err(MediaError(
                "chained Ogg streams with changing format are not supported".into(),
            ));
        }
        let channels = self.info.channels as usize;
        if packet.len() % channels != 0 || packet.len() / channels > MAX_READ_FRAMES {
            return Err(MediaError(
                "Ogg decoder produced an invalid packet size".into(),
            ));
        }
        let mut stereo = Vec::with_capacity((packet.len() / channels).saturating_mul(2));
        for sample in packet.chunks_exact(channels) {
            let left = sample[0] as f32 / 32_768.0;
            let right = if channels == 2 {
                Some(sample[1] as f32 / 32_768.0)
            } else {
                None
            };
            append_stereo_sample(&mut stereo, left, right)?;
        }
        Ok(Some(stereo))
    }
}

impl PcmDecoder for OggDecoder {
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
                let Some(packet) = self.read_packet()? else {
                    break;
                };
                self.pending = packet;
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
        if self.info.total_frames.is_some_and(|total| frame > total) {
            return Err(MediaError(
                "seek position is beyond the end of the file".into(),
            ));
        }
        self.reader
            .seek_absgp_pg(frame)
            .map_err(|error| MediaError(error.to_string()))?;
        self.pending.clear();
        self.pending_offset = 0;
        let mut decoded = Vec::new();
        let mut end_of_page = None;
        while end_of_page.is_none() {
            let Some(packet) = self.read_packet()? else {
                return Err(MediaError("seek reached the end of the Ogg stream".into()));
            };
            decoded.extend(packet);
            if decoded.len() > MAX_READ_FRAMES * 8 {
                return Err(MediaError(
                    "Ogg seek page exceeded the bounded decode limit".into(),
                ));
            }
            end_of_page = self.reader.get_last_absgp();
        }
        let decoded_frames = (decoded.len() / 2) as u64;
        let page_start = end_of_page.unwrap_or(frame).saturating_sub(decoded_frames);
        let page_end = page_start.saturating_add(decoded_frames);
        if frame <= page_end {
            let skip_frames = frame.saturating_sub(page_start) as usize;
            self.pending.extend_from_slice(&decoded[skip_frames * 2..]);
            self.pending_offset = 0;
            self.position = frame;
        } else {
            self.pending.clear();
            self.pending_offset = 0;
            self.position = page_end;
            let mut discard = Vec::new();
            while self.position < frame {
                let remaining = (frame - self.position) as usize;
                let read = self.read_frames(remaining.min(MAX_READ_FRAMES).max(1), &mut discard)?;
                if read == 0 {
                    return Err(MediaError("Ogg seek reached end of file".into()));
                }
            }
        }
        Ok(self.position)
    }
}

fn scan_ogg_duration(path: &Path, target_serial: u32) -> Result<u64, MediaError> {
    let mut file = File::open(path)?;
    let mut last_granule = 0_u64;
    let mut header = [0_u8; 27];
    loop {
        match file.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }
        if &header[..4] != b"OggS" || header[4] != 0 {
            return Err(MediaError("invalid Ogg page header".into()));
        }
        let page_serial = u32::from_le_bytes(header[14..18].try_into().unwrap());
        let granule = u64::from_le_bytes(header[6..14].try_into().unwrap());
        if page_serial == target_serial && granule != u64::MAX {
            last_granule = last_granule.max(granule);
        }
        let segment_count = header[26] as usize;
        let mut lacing = [0_u8; 255];
        file.read_exact(&mut lacing[..segment_count])?;
        let body_len: usize = lacing[..segment_count]
            .iter()
            .map(|length| *length as usize)
            .sum();
        file.seek(SeekFrom::Current(body_len as i64))?;
    }
    if last_granule == 0 {
        return Err(MediaError("Ogg stream has no duration metadata".into()));
    }
    Ok(last_granule)
}
