use super::{validate_media_info, MediaError, MediaInfo};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const MAX_MP3_SEEK_INDEX_ENTRIES: usize = 1_000_000;

#[derive(Clone, Copy)]
pub(super) struct Mp3SeekPoint {
    pub(super) frame: u64,
    pub(super) offset: u64,
}

pub(super) struct Mp3Scan {
    pub(super) info: MediaInfo,
    pub(super) points: Vec<Mp3SeekPoint>,
}

pub(super) fn scan_mp3(path: &Path) -> Result<Mp3Scan, MediaError> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut offset = 0_u64;
    let mut prefix = [0_u8; 10];
    if file.read_exact(&mut prefix).is_ok() && &prefix[..3] == b"ID3" {
        if prefix[6..10].iter().any(|byte| byte & 0x80 != 0) {
            return Err(MediaError("invalid MP3 ID3 tag length".into()));
        }
        let tag_size = ((prefix[6] as u64) << 21)
            | ((prefix[7] as u64) << 14)
            | ((prefix[8] as u64) << 7)
            | prefix[9] as u64;
        offset = 10_u64.saturating_add(tag_size);
        if prefix[5] & 0x10 != 0 {
            offset = offset.saturating_add(10);
        }
    }
    let mut sample_rate = None;
    let mut channels = None;
    let mut total_frames = 0_u64;
    let mut next_index_frame = 0_u64;
    let mut points = Vec::new();
    let mut header = [0_u8; 4];
    while offset.saturating_add(4) <= file_len {
        file.seek(SeekFrom::Start(offset))?;
        if file.read_exact(&mut header).is_err() {
            break;
        }
        let Some(frame) = parse_mp3_header(header) else {
            offset = offset.saturating_add(1);
            continue;
        };
        if let Some(rate) = sample_rate {
            if rate != frame.sample_rate || channels != Some(frame.channels) {
                return Err(MediaError(
                    "MP3 files with changing sample rate or channel layout are not supported"
                        .into(),
                ));
            }
        } else {
            sample_rate = Some(frame.sample_rate);
            channels = Some(frame.channels);
        }
        if total_frames >= next_index_frame && points.len() < MAX_MP3_SEEK_INDEX_ENTRIES {
            points.push(Mp3SeekPoint {
                frame: total_frames,
                offset,
            });
            next_index_frame = total_frames.saturating_add(frame.sample_rate as u64);
        }
        total_frames = total_frames.saturating_add(frame.samples_per_frame as u64);
        offset = offset.saturating_add(frame.frame_bytes as u64);
    }
    let rate = sample_rate
        .ok_or_else(|| MediaError("MP3 file contains no valid Layer III frames".into()))?;
    let channels = channels.unwrap_or(2);
    Ok(Mp3Scan {
        info: validate_media_info(MediaInfo {
            sample_rate: rate,
            channels,
            total_frames: Some(total_frames),
        })?,
        points,
    })
}

pub(super) struct Mp3FrameHeader {
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) samples_per_frame: u16,
    pub(super) frame_bytes: usize,
}

pub(super) fn parse_mp3_header(bytes: [u8; 4]) -> Option<Mp3FrameHeader> {
    let header = u32::from_be_bytes(bytes);
    if header >> 21 != 0x7ff {
        return None;
    }
    let version = ((header >> 19) & 0b11) as u8;
    let layer = ((header >> 17) & 0b11) as u8;
    let bitrate_index = ((header >> 12) & 0b1111) as usize;
    let rate_index = ((header >> 10) & 0b11) as usize;
    if version == 1 || layer != 1 || !(1..=14).contains(&bitrate_index) || rate_index == 3 {
        return None;
    }
    let mpeg1 = version == 3;
    let bitrate = if mpeg1 {
        [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
        ][bitrate_index]
    } else {
        [
            0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
        ][bitrate_index]
    } as u32;
    let base_rates = [44_100, 48_000, 32_000];
    let sample_rate = match version {
        3 => base_rates[rate_index],
        2 => base_rates[rate_index] / 2,
        0 => base_rates[rate_index] / 4,
        _ => return None,
    };
    let padding = ((header >> 9) & 1) as usize;
    let coefficient = if mpeg1 { 144 } else { 72 };
    let frame_bytes = coefficient * bitrate as usize * 1000 / sample_rate as usize + padding;
    let channels = if ((header >> 6) & 0b11) == 3 { 1 } else { 2 };
    Some(Mp3FrameHeader {
        sample_rate,
        channels,
        samples_per_frame: if mpeg1 { 1152 } else { 576 },
        frame_bytes,
    })
}
