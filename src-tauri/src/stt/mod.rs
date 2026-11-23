//! Transcription vocale (dictée, épic N-087…N-097) : trait `SttEngine` et moteur Whisper.
//!
//! Pendant de `tts::engine::TtsEngine` : le moteur est chargé à la demande (le modèle pèse 1,5 Go,
//! ~3 s de chargement, spike N-087) et peut être déchargé après inactivité.

pub mod model;
pub mod whisper;

use crate::tts::engine::TtsLanguage;
use std::fmt;

/// Fréquence d'échantillonnage attendue par Whisper (identique à `audio::micro::TARGET_RATE`).
pub const STT_SAMPLE_RATE: u32 = 16_000;
/// En dessous (100 ms), Whisper refuse l'entrée : on rend une erreur claire plutôt qu'un échec opaque.
pub const MIN_SAMPLES: usize = STT_SAMPLE_RATE as usize / 10;

#[derive(Debug)]
pub enum SttError {
    /// Fichier modèle absent (chemin attendu).
    ModelMissing(String),
    /// Le moteur n'est pas chargé : appeler `load()` d'abord.
    NotLoaded,
    /// Audio trop court pour être transcrit.
    AudioTooShort,
    /// Erreur du moteur (chargement ou inférence).
    Engine(String),
}

impl fmt::Display for SttError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SttError::ModelMissing(p) => write!(f, "Modèle de dictée introuvable : {p}"),
            SttError::NotLoaded => write!(f, "Moteur de dictée non chargé"),
            SttError::AudioTooShort => write!(f, "Enregistrement trop court"),
            SttError::Engine(e) => write!(f, "Erreur du moteur de dictée : {e}"),
        }
    }
}

impl std::error::Error for SttError {}

/// Résultat d'une transcription : texte final (ponctuation et casse natives) et langue retenue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcription {
    pub text: String,
    pub language: TtsLanguage,
}

pub trait SttEngine: Send + Sync {
    fn name(&self) -> &'static str;

    /// Charge le modèle en mémoire (idempotent).
    fn load(&mut self) -> Result<(), SttError>;

    fn is_loaded(&self) -> bool;

    /// Libère le modèle (RAM/VRAM) ; `load()` le recharge à la demande.
    fn unload(&mut self);

    /// Transcrit de l'audio f32 mono 16 kHz. `language: None` = détection automatique FR/EN.
    fn transcribe(
        &self,
        pcm_16k_mono: &[f32],
        language: Option<TtsLanguage>,
    ) -> Result<Transcription, SttError>;
}

/// Choisit français ou anglais d'après les probabilités de Whisper (99 langues) : seules ces deux
/// langues sont supportées, une parole détectée « espagnol » est donc rattachée à la plus probable des deux.
/// En cas d'égalité, le français l'emporte.
pub fn pick_language(p_fr: f32, p_en: f32) -> TtsLanguage {
    if p_en > p_fr {
        TtsLanguage::English
    } else {
        TtsLanguage::French
    }
}

/// Assemble les segments Whisper en un texte unique (espaces normalisés, bords retirés).
pub fn join_segments<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    segments
        .into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn francais_choisi_si_plus_probable() {
        assert_eq!(pick_language(0.9, 0.05), TtsLanguage::French);
    }

    #[test]
    fn anglais_choisi_si_plus_probable() {
        assert_eq!(pick_language(0.1, 0.8), TtsLanguage::English);
    }

    #[test]
    fn egalite_donne_le_francais() {
        assert_eq!(pick_language(0.4, 0.4), TtsLanguage::French);
    }

    #[test]
    fn segments_assembles_sans_espaces_parasites() {
        assert_eq!(
            join_segments([" Le bateau. ", "", "  Les vents.\n"]),
            "Le bateau. Les vents."
        );
        assert_eq!(join_segments(std::iter::empty::<&str>()), "");
    }

    #[test]
    fn erreurs_lisibles() {
        assert!(SttError::AudioTooShort.to_string().contains("court"));
        assert!(SttError::ModelMissing("x".into()).to_string().contains("x"));
    }
}
