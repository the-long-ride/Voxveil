use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use voxveil_media::{MAX_READ_FRAMES, MediaDecoder};

pub struct Stems {
    pub name: String,
    pub sample_rate: u32,
    pub vocal: Vec<f32>,
    pub instrumental: Vec<f32>,
}

impl Stems {
    pub fn frames(&self) -> usize {
        self.vocal.len() / 2
    }

    pub fn mix(&self) -> Vec<f32> {
        self.vocal.iter().zip(&self.instrumental).map(|(v, i)| v + i).collect()
    }

    pub fn duration_seconds(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate)
    }
}

pub fn frame_at(samples: &[f32], n: usize) -> [f32; 2] {
    if 2 * n + 1 < samples.len() { [samples[2 * n], samples[2 * n + 1]] } else { [0.0; 2] }
}

fn decode(path: &Path) -> Result<(u32, Vec<f32>), String> {
    let mut decoder =
        MediaDecoder::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let sample_rate = decoder.info().sample_rate;
    let (mut all, mut chunk) = (Vec::new(), Vec::new());
    loop {
        chunk.clear();
        let frames = decoder
            .read_frames(MAX_READ_FRAMES, &mut chunk)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if frames == 0 {
            break;
        }
        all.extend_from_slice(&chunk);
    }
    Ok((sample_rate, all))
}

pub fn load_stems(vocal: &Path, instrumental: &Path) -> Result<Stems, String> {
    let (vocal_rate, mut vocal_pcm) = decode(vocal)?;
    let (inst_rate, mut inst_pcm) = decode(instrumental)?;
    if vocal_rate != inst_rate {
        return Err(format!(
            "sample rates differ: {vocal_rate} vs {inst_rate}; resample one stem first"
        ));
    }
    let len = vocal_pcm.len().min(inst_pcm.len()) / 2 * 2;
    vocal_pcm.truncate(len);
    inst_pcm.truncate(len);
    let stem = |p: &Path| p.file_stem().map_or("?".into(), |s| s.to_string_lossy().into_owned());
    Ok(Stems {
        name: format!("{}+{}", stem(vocal), stem(instrumental)),
        sample_rate: vocal_rate,
        vocal: vocal_pcm,
        instrumental: inst_pcm,
    })
}

/// Writes 32-bit float stereo WAV for listening checks.
pub fn write_wav(path: &Path, sample_rate: u32, samples: &[f32]) -> Result<(), String> {
    let data_bytes = u32::try_from(samples.len() * 4).map_err(|_| "render too long for WAV")?;
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16_u32.to_le_bytes());
    header.extend_from_slice(&3_u16.to_le_bytes()); // IEEE float
    header.extend_from_slice(&2_u16.to_le_bytes());
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&(sample_rate * 8).to_le_bytes());
    header.extend_from_slice(&8_u16.to_le_bytes());
    header.extend_from_slice(&32_u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_bytes.to_le_bytes());
    let file = File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&header).map_err(|error| error.to_string())?;
    for sample in samples {
        writer.write_all(&sample.to_le_bytes()).map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}
