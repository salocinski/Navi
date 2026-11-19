//! Test de fumée Kokoro (carte N-039, local uniquement).
//!
//! Vérifie, sur la machine de l'humain, que le moteur Kokoro charge les modèles,
//! phonémise et produit des échantillons f32 cohérents.
//!
//! Ignoré par défaut : exécution = `cargo test -- --ignored` (nécessite `models/kokoro/`
//! et `models/phonemizer/` — modèles gitignorés, machine de développement uniquement).

use navi_lib::tts::{KokoroEngine, TtsConfig};
use std::path::PathBuf;

fn models_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("models")
}

#[test]
#[ignore = "nécessite les modèles Kokoro + espeak — machine de développement"]
fn kokoro_charge_et_synthetise_en_francais() {
    let dir = models_dir().join("kokoro");
    let config = TtsConfig::with_dir(&dir);
    let engine =
        KokoroEngine::new(config).expect("création du moteur (modèles/phonémiseur présents)");
    let mut engine = engine;
    navi_lib::tts::TtsEngine::load(&mut engine).expect("chargement du modèle en tâche de fond");
    assert!(navi_lib::tts::TtsEngine::is_loaded(&engine));

    let samples = engine
        .synthesize("Bonjour le monde", Some("ff_siwis"), Some("fr"), 1.0)
        .expect("synthèse Kokoro");
    assert!(!samples.is_empty(), "Kokoro a produit 0 échantillon");
    let peak = samples.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
    let rms = (samples
        .iter()
        .map(|s| (*s as f64) * (*s as f64))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt() as f32;
    eprintln!(
        "[smoke] {} échantillons — peak {:.6} — RMS {:.6}",
        samples.len(),
        peak,
        rms
    );
    let duration_s = samples.len() as f32 / 24_000.0;
    eprintln!("[smoke] durée {:.2} s @24kHz", duration_s);
}
