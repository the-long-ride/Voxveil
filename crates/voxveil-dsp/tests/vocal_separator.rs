mod common;

use common::*;

const FRAMES: usize = 4 * 48_000;

struct Scene {
    voice: Vec<f32>,
    drums: Vec<f32>,
    bass: Vec<f32>,
    side: Vec<f32>,
    mix: Vec<f32>,
}

fn scene() -> Scene {
    let voice = centered(&voice(FRAMES));
    let drums = centered(&drums(FRAMES));
    let bass = centered(&sine(FRAMES, 55.0, 0.2));
    let side = side(&sine(FRAMES, 1_234.0, 0.1));
    let mix = add(&add(&voice, &drums), &add(&bass, &side));
    Scene { voice, drums, bass, side, mix }
}

fn attenuation(component: &[f32], mix: &[f32]) -> f64 {
    let db = attenuation_db(component, &sub(mix, component));
    eprintln!("attenuation {db:.2} dB");
    db
}

#[test]
fn centered_voice_is_removed_deeply() {
    let s = scene();
    let db = attenuation(&s.voice, &s.mix);
    assert!(db >= 20.0, "voice attenuation {db:.1} dB");
}

#[test]
fn centered_drums_lose_little() {
    let s = scene();
    let db = attenuation(&s.drums, &s.mix);
    assert!(db <= 4.5, "drum loss {db:.1} dB");
}

#[test]
fn centered_bass_is_preserved() {
    let s = scene();
    let db = attenuation(&s.bass, &s.mix);
    assert!(db <= 1.0, "bass loss {db:.1} dB");
}

#[test]
fn side_content_is_preserved() {
    let s = scene();
    let db = attenuation(&s.side, &s.mix);
    assert!(db.abs() <= 0.5, "side change {db:.1} dB");
}
