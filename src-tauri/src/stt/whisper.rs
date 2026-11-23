//! Moteur Whisper (whisper.cpp via `whisper-rs`, CUDA) derrière `SttEngine`.

use super::{MIN_SAMPLES, SttEngine, SttError, Transcription, join_segments, pick_language};
use crate::tts::engine::TtsLanguage;
use std::path::PathBuf;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, get_lang_id,
};

/// Modèle retenu au spike N-087 (non quantifié, 1 624 555 275 octets).
pub const MODEL_FILE: &str = "ggml-large-v3-turbo.bin";
/// Threads CPU pour les étapes non GPU (spectrogramme, échantillonnage) : i7 4 cœurs / 8 threads.
const THREADS: usize = 4;

pub struct WhisperEngine {
    model_path: PathBuf,
    ctx: Option<WhisperContext>,
}

impl WhisperEngine {
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            ctx: None,
        }
    }
}

fn engine_err(e: impl std::fmt::Display) -> SttError {
    SttError::Engine(e.to_string())
}

impl SttEngine for WhisperEngine {
    fn name(&self) -> &'static str {
        "whisper-large-v3-turbo"
    }

    // shortcut: pas de vérification SHA-256 ici ni de contrôle que CUDA est réellement actif
    // (whisper.cpp retombe silencieusement sur le CPU, ~80 s par phrase) ; N-094 vérifie l'intégrité
    // au téléchargement, N-096 mesurera le comportement sans GPU.
    fn load(&mut self) -> Result<(), SttError> {
        if self.ctx.is_some() {
            return Ok(());
        }
        if !self.model_path.is_file() {
            return Err(SttError::ModelMissing(
                self.model_path.display().to_string(),
            ));
        }
        let path = self
            .model_path
            .to_str()
            .ok_or_else(|| engine_err("chemin non UTF-8"))?;
        let ctx = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(engine_err)?;
        self.ctx = Some(ctx);
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.ctx.is_some()
    }

    fn unload(&mut self) {
        self.ctx = None;
    }

    fn transcribe(
        &self,
        pcm: &[f32],
        language: Option<TtsLanguage>,
    ) -> Result<Transcription, SttError> {
        let ctx = self.ctx.as_ref().ok_or(SttError::NotLoaded)?;
        if pcm.len() < MIN_SAMPLES {
            return Err(SttError::AudioTooShort);
        }
        let mut state = ctx.create_state().map_err(engine_err)?;

        let language = match language {
            Some(l) => l,
            None => {
                // Détection restreinte au FR/EN : on lit les probabilités des deux langues.
                state.pcm_to_mel(pcm, THREADS).map_err(engine_err)?;
                let (_, probs) = state.lang_detect(0, THREADS).map_err(engine_err)?;
                let p = |code: &str| {
                    get_lang_id(code)
                        .and_then(|id| probs.get(id as usize).copied())
                        .unwrap_or(0.0)
                };
                pick_language(p("fr"), p("en"))
            }
        };

        // Beam 5 : réglage par défaut mesuré au spike N-087.
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        params.set_language(Some(language.as_str()));
        params.set_n_threads(THREADS as i32);
        params.set_no_timestamps(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state.full(params, pcm).map_err(engine_err)?;
        let n = state.full_n_segments();
        let segments: Vec<String> = (0..n)
            .filter_map(|i| state.get_segment(i))
            .filter_map(|s| s.to_str_lossy().ok().map(|c| c.into_owned()))
            .collect();
        Ok(Transcription {
            text: join_segments(segments.iter().map(String::as_str)),
            language,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modele_absent_donne_une_erreur_claire() {
        let mut e = WhisperEngine::new(PathBuf::from("Z:/inexistant/modele.bin"));
        assert!(matches!(e.load(), Err(SttError::ModelMissing(_))));
        assert!(!e.is_loaded());
    }

    #[test]
    fn transcrire_sans_charger_est_refuse() {
        let e = WhisperEngine::new(PathBuf::from("x"));
        assert!(matches!(
            e.transcribe(&[0.0; 16_000], None),
            Err(SttError::NotLoaded)
        ));
    }

    /// Matériel + modèle requis :
    /// `NAVI_STT_MODEL=<ggml-large-v3-turbo.bin> NAVI_STT_WAV=<wav 16 kHz mono s16>
    /// cargo test transcription_reelle -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn transcription_reelle_francais() {
        let model = std::env::var("NAVI_STT_MODEL").expect("NAVI_STT_MODEL");
        let wav = std::fs::read(std::env::var("NAVI_STT_WAV").expect("NAVI_STT_WAV")).unwrap();
        // En-tête WAV PCM canonique de 44 octets (ffmpeg -ar 16000 -ac 1 -c:a pcm_s16le).
        let pcm: Vec<f32> = wav[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b) as f32 / 32768.0)
            .collect();
        let mut e = WhisperEngine::new(PathBuf::from(model));
        let t0 = std::time::Instant::now();
        e.load().expect("chargement");
        println!("chargement : {:?}", t0.elapsed());
        let t1 = std::time::Instant::now();
        let out = e.transcribe(&pcm, None).expect("transcription");
        println!("transcription : {:?} -> {out:?}", t1.elapsed());
        assert_eq!(out.language, TtsLanguage::French);
        assert!(out.text.to_lowercase().contains("bateau"));
        e.unload();
        assert!(!e.is_loaded());
    }
}
