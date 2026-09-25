use std::path::Path;

mod flac;
mod mp3;
mod mp3_scan;
mod ogg;
mod wav;

#[cfg(test)]
mod tests;

pub const MAX_READ_FRAMES: usize = 16_384;
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 192_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaFormat {
    Mp3,
    Wav,
    Flac,
    OggVorbis,
}

impl MediaFormat {
    pub fn from_path(path: &Path) -> Result<Self, &'static str> {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("mp3") => Ok(Self::Mp3),
            Some("wav") => Ok(Self::Wav),
            Some("flac") => Ok(Self::Flac),
            Some("ogg") => Ok(Self::OggVorbis),
            _ => Err("unsupported audio file type; choose MP3, WAV, FLAC, or Ogg/Vorbis"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub total_frames: Option<u64>,
}

#[derive(Debug)]
pub struct MediaError(String);

impl std::fmt::Display for MediaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for MediaError {}

impl From<std::io::Error> for MediaError {
    fn from(error: std::io::Error) -> Self {
        Self(error.to_string())
    }
}

pub(crate) trait PcmDecoder {
    fn info(&self) -> MediaInfo;
    fn position_frames(&self) -> u64;
    fn read_frames(
        &mut self,
        max_frames: usize,
        output: &mut Vec<f32>,
    ) -> Result<usize, MediaError>;
    fn seek(&mut self, frame: u64) -> Result<u64, MediaError>;
}

pub struct MediaDecoder {
    inner: Box<dyn PcmDecoder>,
}

impl MediaDecoder {
    pub fn open(path: &Path) -> Result<Self, MediaError> {
        let format = MediaFormat::from_path(path).map_err(|error| MediaError(error.into()))?;
        let inner: Box<dyn PcmDecoder> = match format {
            MediaFormat::Wav => Box::new(wav::WavDecoder::open(path)?),
            MediaFormat::Flac => Box::new(flac::FlacDecoder::open(path)?),
            MediaFormat::OggVorbis => Box::new(ogg::OggDecoder::open(path)?),
            MediaFormat::Mp3 => Box::new(mp3::Mp3Decoder::open(path)?),
        };
        Ok(Self { inner })
    }

    pub fn info(&self) -> MediaInfo {
        self.inner.info()
    }

    pub fn position_frames(&self) -> u64 {
        self.inner.position_frames()
    }

    /// Decode at most MAX_READ_FRAMES frames into interleaved stereo float PCM.
    /// Mono input is duplicated to stereo; files with more than two channels are rejected.
    pub fn read_frames(
        &mut self,
        max_frames: usize,
        output: &mut Vec<f32>,
    ) -> Result<usize, MediaError> {
        if max_frames == 0 || max_frames > MAX_READ_FRAMES {
            return Err(MediaError(format!(
                "read size must be between 1 and {MAX_READ_FRAMES} frames"
            )));
        }
        self.inner.read_frames(max_frames, output)
    }

    pub fn seek(&mut self, frame: u64) -> Result<u64, MediaError> {
        self.inner.seek(frame)
    }
}

pub(crate) fn validate_media_info(info: MediaInfo) -> Result<MediaInfo, MediaError> {
    if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&info.sample_rate) {
        return Err(MediaError(
            "audio sample rate is outside the supported range".into(),
        ));
    }
    if !(1..=2).contains(&info.channels) {
        return Err(MediaError(
            "only mono and stereo music files are supported".into(),
        ));
    }
    Ok(info)
}

pub(crate) fn finite_sample(sample: f32) -> Result<f32, MediaError> {
    if !sample.is_finite() {
        return Err(MediaError("audio file contains a non-finite sample".into()));
    }
    Ok(sample.clamp(-1.0, 1.0))
}

pub(crate) fn append_stereo_sample(
    output: &mut Vec<f32>,
    left: f32,
    right: Option<f32>,
) -> Result<(), MediaError> {
    let left = finite_sample(left)?;
    let right = finite_sample(right.unwrap_or(left))?;
    output.push(left);
    output.push(right);
    Ok(())
}

pub(crate) fn normalize_interleaved(
    input: &[f32],
    channels: u16,
    output: &mut Vec<f32>,
) -> Result<(), MediaError> {
    output.clear();
    if channels == 1 {
        for sample in input {
            append_stereo_sample(output, *sample, None)?;
        }
    } else if channels == 2 {
        for sample in input {
            output.push(finite_sample(*sample)?);
        }
    } else {
        return Err(MediaError(
            "only mono and stereo music files are supported".into(),
        ));
    }
    Ok(())
}
