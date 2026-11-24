use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Instant;

#[derive(Default)]
struct TestOutput {
    running: bool,
    frames_rendered: Arc<AtomicUsize>,
}

impl PlaybackOutput for TestOutput {
    fn start(&mut self) -> Result<(), String> {
        assert!(!self.running);
        self.running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        assert!(self.running);
        self.running = false;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), String> {
        assert!(!self.running);
        Ok(())
    }

    fn render(&mut self, queue: &mut VecDeque<u8>, _source_eof: bool) -> Result<u64, String> {
        assert!(self.running);
        self.frames_rendered
            .fetch_add(queue.len() / 8, Ordering::Relaxed);
        queue.clear();
        Ok(0)
    }
}

fn wait_for_state(
    state: &Mutex<OwnedPlaybackSnapshot>,
    predicate: impl Fn(&OwnedPlaybackSnapshot) -> bool,
) -> OwnedPlaybackSnapshot {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = state.lock().unwrap().clone();
        if predicate(&snapshot) {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "unexpected playback state: {snapshot:?}"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn seeking_an_ended_track_renders_from_the_new_position() {
    const TOTAL: usize = 4_096;
    const TARGET: usize = 2_048;
    let path = std::env::temp_dir().join(format!("voxveil-playback-{}.wav", std::process::id()));
    let mut wav = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x80\xbb\0\0\0\xee\x02\0\x04\0\x10\0data\0\0\0\0".to_vec();
    wav[4..8].copy_from_slice(&(36 + TOTAL as u32 * 4).to_le_bytes());
    wav[40..44].copy_from_slice(&(TOTAL as u32 * 4).to_le_bytes());
    wav.resize(44 + TOTAL * 4, 0);
    std::fs::write(&path, wav).unwrap();

    let state = Arc::new(Mutex::new(OwnedPlaybackSnapshot::default()));
    let worker_state = Arc::clone(&state);
    let output = TestOutput::default();
    let frames_rendered = Arc::clone(&output.frames_rendered);
    let (tx, rx) = mpsc::channel();
    let worker_path = path.clone();
    let worker = thread::spawn(move || {
        run_playback(
            MediaDecoder::open(&worker_path).unwrap(),
            output,
            0,
            ClassicSuppressionProfile::default(),
            rx,
            worker_state,
        )
    });

    let ended = wait_for_state(&state, |snapshot| snapshot.status == PlaybackStatus::Ended);
    assert_eq!(ended.position_frames, TOTAL as u64);
    let first_rendered = frames_rendered.load(Ordering::Relaxed);
    assert!(first_rendered >= TOTAL);

    tx.send(PlaybackCommand::Seek(TARGET as u64)).unwrap();
    let ended_again = wait_for_state(&state, |snapshot| {
        snapshot.status == PlaybackStatus::Ended
            && frames_rendered.load(Ordering::Relaxed) > first_rendered
    });
    assert_eq!(ended_again.position_frames, TOTAL as u64);
    assert_eq!(
        frames_rendered.load(Ordering::Relaxed),
        first_rendered * 2 - TARGET
    );

    tx.send(PlaybackCommand::Stop).unwrap();
    worker.join().unwrap().unwrap();
    std::fs::remove_file(path).unwrap();
}
