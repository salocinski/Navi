//! Bip de test audio (diagnostic TW-039) : un beep à 440 Hz pendant ~2 s.
//! Usage : cargo run --example audio_beep

use navi_lib::audio::AudioHandle;
use std::time::Duration;

fn main() {
    let player = AudioHandle::new().expect("AudioHandle::new");
    let rate = 24_000usize;
    let duration = Duration::from_secs(2);
    let samples = (0..rate * duration.as_secs() as usize)
        .map(|i| {
            let t = i as f32 / rate as f32;
            (t * 440.0 * std::f32::consts::TAU).sin() * 0.3
        })
        .collect::<Vec<f32>>();
    player.enqueue(samples).expect("enqueue");
    println!("beep en cours (2 s, 440 Hz)…");
    std::thread::sleep(Duration::from_secs(4));
    println!("terminé");
}
