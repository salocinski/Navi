//! Lecture de bout en bout (diagnostic client) — même chaîne que « Lire la sélection ».
//! Usage : cargo run --example read_text

use navi_lib::audio::AudioHandle;
use navi_lib::pipeline::synthesize_segments;
use navi_lib::text::segmentation::segment_text;
use navi_lib::tts::{KokoroEngine, TtsConfig};

fn main() {
    let dir = navi_lib::utils::resolve_models_dir();
    eprintln!("modèles : {}", dir.display());
    let config = TtsConfig::with_dir(&dir);
    let mut engine = KokoroEngine::new(config).expect("moteur");
    navi_lib::tts::TtsEngine::load(&mut engine).expect("chargement");
    eprintln!("modèle chargé");

    let audio = AudioHandle::new().expect("audio");
    audio.reset().expect("reset");

    let text = "Domio est une application d'accompagnement à l'entretien résidentiel qui aide les propriétaires à planifier, comprendre et suivre l'entretien de leur propriété tout au long de l'année.";
    let segs = segment_text(text);
    eprintln!("segments : {}", segs.len());

    let n = synthesize_segments(
        &engine,
        &segs,
        &audio,
        &|| false,
        Some(navi_lib::tts::engine::TtsLanguage::French),
        1.0,
        &|| (),
    )
    .expect("synthèse");
    eprintln!("synthétisés {n} segments, lecture…");
    std::thread::sleep(std::time::Duration::from_secs(12));
    eprintln!("terminé");
}
