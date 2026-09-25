use super::*;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

fn test_file(extension: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "voxveil-media-{}-{}.{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed),
        extension
    ))
}

fn write_pcm_wav(path: &Path, channels: u16, samples: &[i16]) {
    let mut writer = WavWriter::create(
        path,
        WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )
    .unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn identifies_only_the_supported_common_music_formats() {
    assert_eq!(
        MediaFormat::from_path(Path::new("track.MP3")),
        Ok(MediaFormat::Mp3)
    );
    assert_eq!(
        MediaFormat::from_path(Path::new("track.wav")),
        Ok(MediaFormat::Wav)
    );
    assert_eq!(
        MediaFormat::from_path(Path::new("track.flac")),
        Ok(MediaFormat::Flac)
    );
    assert_eq!(
        MediaFormat::from_path(Path::new("track.ogg")),
        Ok(MediaFormat::OggVorbis)
    );
    assert!(MediaFormat::from_path(Path::new("track.m4a")).is_err());
    assert!(MediaFormat::from_path(Path::new("track")).is_err());
}

#[test]
fn wav_decoder_normalizes_mono_and_bounds_each_read() {
    let path = test_file("wav");
    write_pcm_wav(&path, 1, &[-32_768, 0, 32_767]);

    let mut decoder = MediaDecoder::open(&path).unwrap();
    assert_eq!(decoder.info().sample_rate, 48_000);
    assert_eq!(decoder.info().channels, 1);
    assert_eq!(decoder.info().total_frames, Some(3));
    assert!(decoder
        .read_frames(MAX_READ_FRAMES + 1, &mut Vec::new())
        .is_err());

    let mut output = Vec::new();
    assert_eq!(decoder.read_frames(2, &mut output).unwrap(), 2);
    assert_eq!(output, vec![-1.0, -1.0, 0.0, 0.0]);
    assert_eq!(decoder.read_frames(2, &mut output).unwrap(), 1);
    assert_eq!(output, vec![32_767.0 / 32_768.0, 32_767.0 / 32_768.0]);
    assert_eq!(decoder.read_frames(2, &mut output).unwrap(), 0);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn wav_seek_repositions_by_frame_and_rejects_unsafe_channel_layouts() {
    let path = test_file("wav");
    write_pcm_wav(&path, 2, &[1, 2, 3, 4, 5, 6]);
    let mut decoder = MediaDecoder::open(&path).unwrap();
    assert_eq!(decoder.seek(2).unwrap(), 2);
    let mut output = Vec::new();
    assert_eq!(decoder.read_frames(1, &mut output).unwrap(), 1);
    assert_eq!(output, vec![5.0 / 32_768.0, 6.0 / 32_768.0]);
    assert!(decoder.seek(4).is_err());
    std::fs::remove_file(path).unwrap();

    let surround = test_file("wav");
    write_pcm_wav(&surround, 3, &[0; 6]);
    assert!(MediaDecoder::open(&surround).is_err());
    std::fs::remove_file(surround).unwrap();
}

#[test]
fn decoder_rejects_file_content_that_does_not_match_its_extension() {
    let path = test_file("mp3");
    write_pcm_wav(&path, 2, &[0; 8]);
    assert!(MediaDecoder::open(&path).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn parses_mpeg1_layer_three_frame_metadata() {
    let header = mp3_scan::parse_mp3_header([0xff, 0xfb, 0x90, 0x64]).unwrap();
    assert_eq!(header.sample_rate, 44_100);
    assert_eq!(header.channels, 2);
    assert_eq!(header.samples_per_frame, 1_152);
    assert_eq!(header.frame_bytes, 417);
    assert!(mp3_scan::parse_mp3_header([0x7f, 0xfb, 0x90, 0x64]).is_none());
    assert!(mp3_scan::parse_mp3_header([0xff, 0xfb, 0xfc, 0x64]).is_none());
}
