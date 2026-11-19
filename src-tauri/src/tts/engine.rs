//! Interface du moteur TTS — isolation pour permettre le remplacement du moteur.
//!
//! Objectif (carte N-009) :
//! - définir un contrat stable `TtsEngine` (chargement paresseux, synthèse, voix, langues) ;
//! - l'utiliser pour le moteur Kokoro (impl ci-dessous) ;
//! - vérifier qu'il tient pour Piper / MeloTTS (voir `docs/s1-09-tts-engine.md`).
//!
//! Le moteur reste **synchrone** (appelant responsable du thread et de la file).
//! L'audio produit est du `f32` mono à 24 kHz (Kokoro ; Piper 16 kHz possible via `sample_rate`).

use std::fmt;

/// Fréquence d'échantillonnage produite par les moteurs supportés/cible.
pub const TTS_SAMPLE_RATE: u32 = 24_000;

/// Langues supportées par le produit à ce stade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsLanguage {
    French,
    English,
}

impl TtsLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsLanguage::French => "fr",
            TtsLanguage::English => "en",
        }
    }

    /// Traduit le libellé produit (« auto », « fr », « en », « fr-fr », « en-us »…)
    /// en langue cible ; `None` si inconnu ou non supporté.
    pub fn from_str_lenient(value: &str) -> Option<Self> {
        let v = value.to_lowercase();
        if v.starts_with("fr") {
            Some(TtsLanguage::French)
        } else if v.starts_with("en") {
            Some(TtsLanguage::English)
        } else {
            None
        }
    }
}

impl fmt::Display for TtsLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Demande de synthèse.
#[derive(Debug, Clone)]
pub struct SynthesisRequest {
    pub text: String,
    /// Identifiant de voix (ex. « af_heart », « ff_siwis », Piper « fr_FR-siwis-medium »).
    /// `None` → voix par défaut du moteur.
    pub voice: Option<String>,
    /// Langue ; `None` → langue par défaut du moteur.
    pub language: Option<TtsLanguage>,
    /// Vitesse de lecture (1.0 = normale).
    pub speed: f32,
}

impl Default for SynthesisRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            voice: None,
            language: None,
            speed: 1.0,
        }
    }
}

/// Résultat de synthèse — prêt pour la file audio (`SegmentQueue`).
#[derive(Debug, Clone)]
pub struct SynthesisOutput {
    /// Échantillons `f32` mono.
    pub samples: Vec<f32>,
    /// Fréquence d'échantillonnage de `samples`.
    pub sample_rate: u32,
}

/// Erreurs du moteur, stables pour l'UI.
#[derive(Debug)]
pub enum TtsError {
    /// Le moteur n'est pas chargé (chargement en tâche de fond pas terminé).
    NotLoaded,
    /// Langue demandée non supportée par le moteur.
    UnsupportedLanguage(String),
    /// Voix demandée inconnue du moteur.
    UnknownVoice(String),
    /// Modèle/poids introuvables au chemin configuré.
    ModelMissing(String),
    /// Échec de phonémisation.
    Phonemization(String),
    /// Échec d'inférence ONNX.
    Inference(String),
    /// Erreur d'entrée/sortie (modèle, voix, vocab…).
    Io(String),
}

impl fmt::Display for TtsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TtsError::NotLoaded => write!(
                f,
                "Moteur non chargé — le chargement est en cours en tâche de fond"
            ),
            TtsError::UnsupportedLanguage(lang) => {
                write!(f, "Langue non supportée par ce moteur : {lang}")
            }
            TtsError::UnknownVoice(voice) => write!(f, "Voix inconnue de ce moteur : {voice}"),
            TtsError::ModelMissing(path) => write!(
                f,
                "Modèle introuvable au chemin {path} — vérifiez l'installation des modèles"
            ),
            TtsError::Phonemization(detail) => write!(f, "Phonémisation impossible : {detail}"),
            TtsError::Inference(detail) => write!(f, "Inférence ONNX échouée : {detail}"),
            TtsError::Io(detail) => write!(f, "Erreur d'accès aux fichiers du moteur : {detail}"),
        }
    }
}

impl std::error::Error for TtsError {}

/// Contrat du moteur TTS.
///
/// Implémentations attendues : Kokoro (actuel), Piper (réserve), MeloTTS (réserve).
pub trait TtsEngine: Send + Sync {
    /// Nom stable du moteur (pour logs/diagnostic, jamais pour l'UI utilisateur).
    fn name(&self) -> &'static str;

    /// Langues supportées.
    fn supported_languages(&self) -> &'static [TtsLanguage];

    /// Identifiants de voix exposés par le moteur (peut être vide tant que non chargé).
    fn voices(&self) -> Vec<String>;

    /// Charge le modèle (paresseux côté Kokoro, via `ensure_session`).
    /// Peut être appelé plusieurs fois ; idempotent.
    fn load(&mut self) -> Result<(), TtsError>;

    /// Vrai si le moteur est prêt à synthétiser.
    fn is_loaded(&self) -> bool;

    /// Synthétise `request.text` en échantillons `f32` mono.
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisOutput, TtsError>;
}

/// Implémentation Kokoro (déléguée à `super::KokoroEngine`).
impl TtsEngine for super::KokoroEngine {
    fn name(&self) -> &'static str {
        "kokoro-82m"
    }

    fn supported_languages(&self) -> &'static [TtsLanguage] {
        &[TtsLanguage::French, TtsLanguage::English]
    }

    fn voices(&self) -> Vec<String> {
        self.voice_styles
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn load(&mut self) -> Result<(), TtsError> {
        self.ensure_session()
            .map_err(|e| TtsError::Inference(e.to_string()))
    }

    fn is_loaded(&self) -> bool {
        self.session.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisOutput, TtsError> {
        let language = request
            .language
            .map(|l| l.as_str().to_string())
            .or_else(|| {
                let default = self.config.default_language.as_str();
                // « auto » = détection non encore branchée (carte N-036) → laisse le moteur choisir.
                (!default.eq_ignore_ascii_case("auto")).then(|| default.to_string())
            });

        // Langue non supportée → erreur claire (pas d'anglais silencieux pour du français).
        if let Some(lang) = &language {
            if TtsLanguage::from_str_lenient(lang).is_none() {
                return Err(TtsError::UnsupportedLanguage(lang.clone()));
            }
        }

        let samples = self
            .synthesize(
                &request.text,
                request.voice.as_deref(),
                language.as_deref(),
                request.speed,
            )
            .map_err(|e| TtsError::Inference(e.to_string()))?;

        Ok(SynthesisOutput {
            samples,
            sample_rate: TTS_SAMPLE_RATE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Moteur de test minimal (aucune ressource, aucune latence).
    struct StubEngine {
        loaded: std::sync::Arc<std::sync::Mutex<bool>>,
    }

    impl StubEngine {
        fn new() -> Self {
            Self {
                loaded: std::sync::Arc::new(std::sync::Mutex::new(false)),
            }
        }
    }

    impl TtsEngine for StubEngine {
        fn name(&self) -> &'static str {
            "stub"
        }

        fn supported_languages(&self) -> &'static [TtsLanguage] {
            &[TtsLanguage::French, TtsLanguage::English]
        }

        fn voices(&self) -> Vec<String> {
            vec!["stub-voice".to_string()]
        }

        fn load(&mut self) -> Result<(), TtsError> {
            *self.loaded.lock().unwrap() = true;
            Ok(())
        }

        fn is_loaded(&self) -> bool {
            *self.loaded.lock().unwrap()
        }

        fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisOutput, TtsError> {
            if !self.is_loaded() {
                return Err(TtsError::NotLoaded);
            }
            if request.text.trim().is_empty() {
                return Err(TtsError::Phonemization("texte vide".to_string()));
            }
            Ok(SynthesisOutput {
                samples: vec![0.0; 1024],
                sample_rate: TTS_SAMPLE_RATE,
            })
        }
    }

    #[test]
    fn contrat_charge_puis_synthetise() {
        let mut engine = StubEngine::new();
        assert!(!engine.is_loaded());
        let before = engine.synthesize(&SynthesisRequest::default());
        assert!(matches!(before, Err(TtsError::NotLoaded)));

        engine.load().unwrap();
        assert!(engine.is_loaded());

        let request = SynthesisRequest {
            text: "bonjour le monde".to_string(),
            voice: Some("stub-voice".to_string()),
            language: Some(TtsLanguage::French),
            speed: 1.0,
        };
        let out = engine.synthesize(&request).unwrap();
        assert_eq!(out.sample_rate, 24_000);
        assert_eq!(out.samples.len(), 1024);
    }

    #[test]
    fn langue_inconnue_est_refusee() {
        assert!(TtsLanguage::from_str_lenient("auto").is_none());
        assert_eq!(
            TtsLanguage::from_str_lenient("fr-FR"),
            Some(TtsLanguage::French)
        );
        assert_eq!(
            TtsLanguage::from_str_lenient("en-US"),
            Some(TtsLanguage::English)
        );
        assert!(TtsLanguage::from_str_lenient("de").is_none());
    }

    #[test]
    fn request_par_defaut_est_coherent() {
        let r = SynthesisRequest::default();
        assert_eq!(r.speed, 1.0);
        assert!(r.voice.is_none());
        assert!(r.language.is_none());
    }
}
