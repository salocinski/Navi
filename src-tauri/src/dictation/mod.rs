//! Dictée vocale (carte N-092) : machine d'états et orchestration appui maintenu → micro → Whisper → saisie.
//!
//! Séquence : l'appui du raccourci arrête la lecture (évite l'écho micro) et démarre l'enregistrement ;
//! le relâchement (ou le plafond de 60 s) lance la transcription puis la saisie dans la fenêtre active.

use crate::audio::micro::MicCapture;
use crate::hotkey::hold::{HoldEnd, HoldGuard, spawn_hold_watch};
use crate::insert::insert_text;
use crate::state::PlaybackState;
use crate::stt::model::{self, ModelState};
use crate::stt::whisper::WhisperEngine;
use crate::stt::{MIN_SAMPLES, SttEngine};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// En dessous, l'appui est un effleurement accidentel : ni transcription ni erreur.
const MIN_HOLD: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DictationState {
    Idle,
    /// Micro ouvert, appui maintenu (pastille REC).
    Recording,
    /// Transcription en cours (chargement du modèle compris).
    Transcribing,
    /// Saisie du texte dans la fenêtre active.
    Inserting,
    /// Échec (message dans `error_message`) ; revient à `Idle` à la prochaine dictée.
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DictationEvent {
    PhaseChanged {
        from: DictationState,
        to: DictationState,
    },
    Error {
        message: String,
    },
    /// Téléchargement du modèle : octets reçus / total (émis à chaque pour cent).
    ModelDownload {
        done: u64,
        total: u64,
    },
    ModelReady,
    ModelFailed {
        message: String,
    },
}

type Listener = Arc<dyn Fn(DictationEvent) + Send + Sync>;

struct Inner {
    state: DictationState,
    error: String,
    listeners: Vec<Listener>,
}

#[derive(Clone)]
pub struct DictationMachine(Arc<Mutex<Inner>>);

impl Default for DictationMachine {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Inner {
            state: DictationState::Idle,
            error: String::new(),
            listeners: Vec::new(),
        })))
    }
}

impl DictationMachine {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn state(&self) -> DictationState {
        self.lock().state
    }

    /// Enregistrement, transcription ou saisie en cours.
    pub fn is_active(&self) -> bool {
        matches!(
            self.state(),
            DictationState::Recording | DictationState::Transcribing | DictationState::Inserting
        )
    }

    pub fn error_message(&self) -> String {
        self.lock().error.clone()
    }

    pub fn subscribe(&self, listener: impl Fn(DictationEvent) + Send + Sync + 'static) {
        self.lock().listeners.push(Arc::new(listener));
    }

    pub fn emit(&self, event: DictationEvent) {
        let listeners = self.lock().listeners.clone();
        for l in &listeners {
            l(event.clone());
        }
    }

    /// Change d'état si `allowed(état actuel)` ; notifie hors verrou.
    fn transition(
        &self,
        to: DictationState,
        allowed: impl Fn(DictationState) -> bool,
        error: Option<String>,
    ) -> Result<(), String> {
        let (from, listeners) = {
            let mut inner = self.lock();
            let from = inner.state;
            if !allowed(from) {
                return Err(format!("Transition de dictée refusée : {from:?} → {to:?}"));
            }
            inner.state = to;
            if let Some(e) = &error {
                inner.error = e.clone();
            }
            (from, inner.listeners.clone())
        };
        if from != to {
            for l in &listeners {
                l(DictationEvent::PhaseChanged { from, to });
            }
        }
        if let Some(message) = error {
            for l in &listeners {
                l(DictationEvent::Error {
                    message: message.clone(),
                });
            }
        }
        Ok(())
    }

    /// Idle | Error → Recording. Refusé si une dictée est déjà active.
    pub fn begin(&self) -> Result<(), String> {
        self.transition(
            DictationState::Recording,
            |s| matches!(s, DictationState::Idle | DictationState::Error),
            None,
        )
    }

    pub fn to_transcribing(&self) -> Result<(), String> {
        self.transition(
            DictationState::Transcribing,
            |s| s == DictationState::Recording,
            None,
        )
    }

    pub fn to_inserting(&self) -> Result<(), String> {
        self.transition(
            DictationState::Inserting,
            |s| s == DictationState::Transcribing,
            None,
        )
    }

    /// Fin normale (ou abandon silencieux) depuis un état actif.
    pub fn finish(&self) {
        let _ = self.transition(DictationState::Idle, |s| s != DictationState::Idle, None);
    }

    pub fn fail(&self, message: String) {
        let _ = self.transition(DictationState::Error, |_| true, Some(message));
    }

    /// Efface l'erreur affichée (pastille) sans toucher à une dictée redémarrée entre-temps.
    pub fn clear_error(&self) {
        let _ = self.transition(DictationState::Idle, |s| s == DictationState::Error, None);
    }
}

/// Dictée partagée : machine d'états + moteur Whisper (chargé à la demande) + état du modèle.
#[derive(Clone, Default)]
pub struct Dictation {
    pub machine: DictationMachine,
    engine: Arc<Mutex<Option<WhisperEngine>>>,
    /// Empreinte du fichier modèle déjà contrôlée dans ce processus (le hachage prend ~4 s).
    verified: Arc<AtomicBool>,
    downloading: Arc<AtomicBool>,
}

impl Dictation {
    pub fn is_downloading(&self) -> bool {
        self.downloading.load(Ordering::SeqCst)
    }

    /// Lance le téléchargement du modèle en tâche de fond ; `false` s'il est déjà en cours.
    pub fn start_download(&self) -> bool {
        if self.downloading.swap(true, Ordering::SeqCst) {
            return false;
        }
        let this = self.clone();
        std::thread::spawn(move || {
            let mut last_pct = u64::MAX;
            let result = model::download(&model::model_path(), |done, total| {
                let pct = done * 100 / total;
                if pct != last_pct {
                    last_pct = pct;
                    this.machine
                        .emit(DictationEvent::ModelDownload { done, total });
                }
            });
            this.verified.store(result.is_ok(), Ordering::SeqCst);
            this.downloading.store(false, Ordering::SeqCst);
            match result {
                Ok(()) => this.machine.emit(DictationEvent::ModelReady),
                Err(message) => {
                    eprintln!("[navi] modèle de dictée : {message}");
                    this.machine.emit(DictationEvent::ModelFailed { message });
                }
            }
        });
        true
    }

    /// Modèle utilisable ou erreur lisible ; lance le téléchargement s'il est absent ou corrompu.
    /// Le modèle fourni par `NAVI_STT_MODEL` n'est ni contrôlé ni remplacé.
    fn ensure_model(&self, path: &std::path::Path) -> Result<(), String> {
        if model::is_overridden() || self.verified.load(Ordering::SeqCst) {
            return Ok(());
        }
        if self.is_downloading() {
            return Err("Modèle de dictée en cours de téléchargement".into());
        }
        let reason = match model::verify(path)? {
            ModelState::Valid => {
                self.verified.store(true, Ordering::SeqCst);
                return Ok(());
            }
            ModelState::Missing => "absent",
            ModelState::Corrupt => "corrompu",
        };
        self.start_download();
        Err(format!(
            "Modèle de dictée {reason} : téléchargement lancé (1,5 Go), à réessayer une fois terminé"
        ))
    }
}

/// Démarre une dictée sur appui du raccourci. `guard` est libéré au relâchement.
pub fn start(view: &crate::NaviRef, vk: u16, guard: Arc<HoldGuard>) {
    let dictation = view.dictation.clone();
    if dictation.machine.is_active() {
        guard.end();
        return;
    }
    // Lecture à voix haute arrêtée avant d'ouvrir le micro (pas d'écho) ; avant `begin`, car l'arrêt
    // de la lecture masque la pastille.
    if matches!(
        view.machine.state(),
        PlaybackState::Capturing
            | PlaybackState::Generating
            | PlaybackState::Playing
            | PlaybackState::Paused
    ) {
        let _ = crate::stop_reading_impl(view);
    }
    crate::touch_user_activity();
    if dictation.machine.begin().is_err() {
        guard.end();
        return;
    }
    let config = crate::config::AppConfig::load().unwrap_or_default();
    let capture = match MicCapture::start(config.microphone.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            dictation.machine.fail(e);
            guard.end();
            return;
        }
    };
    spawn_hold_watch(vk, guard, move |end, held| {
        process(&dictation, capture, end, held)
    });
}

fn process(dictation: &Dictation, capture: MicCapture, end: HoldEnd, held: Duration) {
    let pcm = capture.stop();
    eprintln!("[navi] dictée : {end:?} après {} ms", held.as_millis());
    if held < MIN_HOLD || pcm.len() < MIN_SAMPLES {
        dictation.machine.finish();
        return;
    }
    if dictation.machine.to_transcribing().is_err() {
        return;
    }
    let text = match transcribe(dictation, &pcm) {
        Ok(t) => t,
        Err(e) => return dictation.machine.fail(e),
    };
    if text.is_empty() {
        dictation.machine.finish();
        return;
    }
    if dictation.machine.to_inserting().is_err() {
        return;
    }
    match insert_text(&text) {
        Ok(_) => dictation.machine.finish(),
        Err(e) => dictation.machine.fail(e.to_string()),
    }
}

fn transcribe(dictation: &Dictation, pcm: &[f32]) -> Result<String, String> {
    let path = model::model_path();
    dictation.ensure_model(&path)?;
    let mut guard = dictation
        .engine
        .lock()
        .map_err(|_| "verrou moteur de dictée".to_string())?;
    let engine = guard.get_or_insert_with(|| WhisperEngine::new(path));
    engine.load().map_err(|e| e.to_string())?;
    let out = engine.transcribe(pcm, None).map_err(|e| e.to_string())?;
    Ok(out.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn phases(m: &DictationMachine) -> Arc<Mutex<Vec<(DictationState, DictationState)>>> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        m.subscribe(move |e| {
            if let DictationEvent::PhaseChanged { from, to } = e {
                s.lock().unwrap().push((from, to));
            }
        });
        seen
    }

    #[test]
    fn cycle_nominal() {
        let m = DictationMachine::default();
        let seen = phases(&m);
        m.begin().unwrap();
        assert!(m.is_active());
        m.to_transcribing().unwrap();
        m.to_inserting().unwrap();
        m.finish();
        assert_eq!(m.state(), DictationState::Idle);
        assert!(!m.is_active());
        assert_eq!(seen.lock().unwrap().len(), 4);
    }

    #[test]
    fn deuxieme_dictee_refusee_pendant_une_dictee() {
        let m = DictationMachine::default();
        m.begin().unwrap();
        assert!(m.begin().is_err());
        m.to_transcribing().unwrap();
        assert!(m.begin().is_err());
        assert!(m.to_inserting().is_ok());
        assert!(m.to_transcribing().is_err());
    }

    #[test]
    fn erreur_puis_nouvelle_dictee_possible() {
        let m = DictationMachine::default();
        let errors = Arc::new(AtomicUsize::new(0));
        let e = errors.clone();
        m.subscribe(move |ev| {
            if matches!(ev, DictationEvent::Error { .. }) {
                e.fetch_add(1, Ordering::SeqCst);
            }
        });
        m.begin().unwrap();
        m.fail("micro absent".into());
        assert_eq!(m.state(), DictationState::Error);
        assert_eq!(m.error_message(), "micro absent");
        assert!(!m.is_active());
        assert_eq!(errors.load(Ordering::SeqCst), 1);
        m.begin().unwrap();
        assert_eq!(m.state(), DictationState::Recording);
    }

    #[test]
    fn clear_error_ne_touche_pas_une_dictee_redemarree() {
        let m = DictationMachine::default();
        m.begin().unwrap();
        m.fail("x".into());
        m.begin().unwrap();
        m.clear_error();
        assert_eq!(m.state(), DictationState::Recording);
        m.finish();
        m.fail("y".into());
        m.clear_error();
        assert_eq!(m.state(), DictationState::Idle);
    }

    #[test]
    fn finish_au_repos_est_sans_effet() {
        let m = DictationMachine::default();
        let seen = phases(&m);
        m.finish();
        assert!(seen.lock().unwrap().is_empty());
    }
}
