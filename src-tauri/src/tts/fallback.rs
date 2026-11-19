//! Repli automatique Qwen3-TTS → Kokoro (carte N-071).
//!
//! Moteur par défaut = **Qwen3-TTS-0.6B-CustomVoice** (voix officielle Ryan, carte N-070).
//! Si Qwen est indisponible — torch absent, venv non trouvé, modèle non téléchargé,
//! pas de CUDA (CPU automatique côté sidecar), ou panne en cours de synthèse — le
//! moteur bascule sur **Kokoro** sans interrompre la lecture.
//!
//! Sélection via `NAVI_TTS_ENGINE` : `auto` (défaut, Qwen puis repli Kokoro),
//! `qwen` (Qwen obligatoire), `kokoro` (Kokoro direct).

use super::engine::{SynthesisOutput, SynthesisRequest, TtsEngine, TtsError, TtsLanguage};
use super::qwen::{QwenConfig, QwenTtsEngine};
use super::{KokoroEngine, TtsConfig};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct ResilientEngine {
    /// `Some` si Qwen est tenté (mode auto/qwen) ; `None` si `NAVI_TTS_ENGINE=kokoro`.
    qwen: Option<QwenTtsEngine>,
    kokoro: KokoroEngine,
    /// Vrai tant que Qwen répond ; passe à faux à la première panne → repli Koukoro.
    qwen_active: AtomicBool,
}

impl ResilientEngine {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let preference = std::env::var("NAVI_TTS_ENGINE").unwrap_or_else(|_| "auto".to_string());
        let qwen = match preference.to_lowercase().as_str() {
            "kokoro" => None,
            "qwen" => Some(QwenTtsEngine::new(QwenConfig::from_env())),
            _ => Some(QwenTtsEngine::new(QwenConfig::from_env())), // auto
        };

        let dir = crate::utils::resolve_models_dir();
        let kokoro = KokoroEngine::new(TtsConfig::with_dir(&dir))?;
        Ok(Self { qwen, kokoro, qwen_active: AtomicBool::new(false) })
    }

    /// Vrai si le moteur Qwen est le moteur actif (sinon Kokoro sert de secours).
    pub fn qwen_active(&self) -> bool {
        self.qwen_active.load(Ordering::SeqCst)
    }

    /// Nombre de moteurs ; Qwen et Kokoro exposent les mêmes langues.
    fn both_languages(&self) -> &'static [TtsLanguage] {
        &[TtsLanguage::French, TtsLanguage::English]
    }
}

impl TtsEngine for ResilientEngine {
    fn name(&self) -> &'static str {
        if self.qwen_active() { "qwen3-tts-0.6b-customvoice" } else { "kokoro-82m" }
    }

    fn supported_languages(&self) -> &'static [TtsLanguage] {
        self.both_languages()
    }

    fn voices(&self) -> Vec<String> {
        if self.qwen_active() {
            match &self.qwen {
                Some(qwen) => qwen.voices(),
                None => vec![],
            }
        } else {
            self.kokoro.voices()
        }
    }

    fn load(&mut self) -> Result<(), TtsError> {
        // 1. Tenter Qwen si activé ; toute erreur bascule sur Kokoro (secours).
        if let Some(qwen) = &mut self.qwen {
            match TtsEngine::load(qwen) {
                Ok(()) => {
                    self.qwen_active.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("[navi] moteur qwen indisponible — repli Kokoro : {e}");
                }
            }
        }
        // 2. Repli Kokoro (chargement paresseux possible à la 1re synthèse).
        self.kokoro.ensure_session().map_err(|e| TtsError::Inference(e.to_string()))
    }

    fn is_loaded(&self) -> bool {
        if self.qwen_active() { true } else { self.kokoro.is_loaded() }
    }

    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisOutput, TtsError> {
        // Qwen d'abord (si il était chargé) ; toute panne → bascule Kokoro définitive.
        if self.qwen_active() {
            if let Some(qwen) = &self.qwen {
                match qwen.synthesize(request) {
                    Ok(out) => return Ok(out),
                    Err(e) => {
                        eprintln!("[navi] synthèse Qwen échouée — repli Kokoro : {e}");
                        self.qwen_active.store(false, Ordering::SeqCst);
                    }
                }
            }
        }
        // Repli Kokoro : chargement paresseux puis synthèse via le contrat TtsEngine.
        self.kokoro.ensure_session().map_err(|e| TtsError::Inference(e.to_string()))?;
        TtsEngine::synthesize(&self.kokoro, request)
    }
}
