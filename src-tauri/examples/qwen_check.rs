//! Vérification du moteur Qwen3-TTS (carte N-071) — mesure RTF sur texte FR et EN.
//!
//! Usage :
//!   NAVI_QWEN_PYTHON=<venv>/python cargo run --example qwen_check
//!
//! Synthétise deux phrases (voix Ryan, FR puis EN) et affiche le RTF (real-time factor) :
//! RTF < 1 = la synthèse est plus rapide que la lecture. Écrit deux WAV 16 bits mono.

use navi_lib::tts::{QwenConfig, QwenTtsEngine, TtsEngine, SynthesisRequest, TtsLanguage};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

const FR: &str = "Domio est une application d'accompagnement à l'entretien résidentiel qui aide les propriétaires à planifier, comprendre et suivre l'entretien de leur propriété tout au long de l'année.";
const EN: &str = "Domio is a home maintenance companion application that helps homeowners plan, understand and keep track of their property maintenance throughout the year.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = QwenConfig::from_env();
    println!("python  : {}", config.python.display());
    println!("sidecar : {}", config.sidecar.display());
    println!("voix    : {}", config.speaker);
    println!("chargement du modèle…");
    let started = Instant::now();
    let mut engine = QwenTtsEngine::new(config);
    TtsEngine::load(&mut engine).map_err(|e| format!("chargement impossible : {e}"))?;
    println!("modèle prêt en {:.1} s\n", started.elapsed().as_secs_f32());

    for (lang, text) in [(TtsLanguage::French, FR), (TtsLanguage::English, EN)] {
        let label = match lang {
            TtsLanguage::French => "FR",
            TtsLanguage::English => "EN",
        };
        let t0 = Instant::now();
        let out = TtsEngine::synthesize(
            &engine,
            &SynthesisRequest { text: text.to_string(), voice: None, language: Some(lang), speed: 1.0 },
        )
        .map_err(|e| format!("synthèse {label} échouée : {e}"))?;
        let synth_s = t0.elapsed().as_secs_f32();
        let audio_s = out.samples.len() as f32 / out.sample_rate as f32;
        let rtf = synth_s / audio_s;
        let path = format!("qwen_check_{label}.wav");
        write_wav_s16(Path::new(&path), &out.samples, out.sample_rate)?;
        println!("{label} : {:.2} s audio en {:.1} s — RTF = {:.2} → {}", audio_s, synth_s, rtf, path);
    }
    Ok(())
}

fn write_wav_s16(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    let data_len = (samples.len() * 2) as u32;
    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_len as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_len).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        buf.extend_from_slice(&v.to_le_bytes());
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(&buf)?;
    Ok(())
}
