//! Machine d'états de lecture — contrat capture → états → audio → UI
//!
//! Etats (carte N-032) : Inactif / Capture / Génération / Lecture / Pause / Arrêté / Erreur.
//! La machine est un composant **synchrone et sûr** : les transitions invalides
//! retournent une erreur claire, jamais un panic. Les écouteurs sont appelés
//! **synchronement** lors des transitions — ils doivent rester non bloquants
//! (ex. émettre un événement Tauri). La file d'erreurs/progression est drainable
//! via `drain_events` si l'appelant préfère un découplé par pompe.
//!
//! Un arrêt invalide le jeton de génération courant : un worker TTS qui vérifie
//! `is_generation_cancelled(id)` entre deux segments doit abandonner sans
//! écrire dans la file audio.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

/// Phases du produit — contrat stable pour l'UI (sérialisée telle quelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    /// Inactif (ou après Stop avant toute nouvelle lecture).
    Idle,
    /// Capture du texte en cours (simulation Ctrl+C + snapshot/restauration).
    Capturing,
    /// Génération audio en cours (segments pas encore en lecture).
    Generating,
    /// Lecture audio en cours.
    Playing,
    /// Lecture en pause (position conservée).
    Paused,
    /// Arrêt explicite (arrêt de la génération et de la file).
    Stopped,
    /// Erreur terminale (message dans `error_message`).
    Error,
}

impl fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaybackState::Idle => f.write_str("inactif"),
            PlaybackState::Capturing => f.write_str("capture"),
            PlaybackState::Generating => f.write_str("génération"),
            PlaybackState::Playing => f.write_str("lecture"),
            PlaybackState::Paused => f.write_str("pause"),
            PlaybackState::Stopped => f.write_str("arrêté"),
            PlaybackState::Error => f.write_str("erreur"),
        }
    }
}

/// Événements émis vers l'UI (sérialisés pour Tauri).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StateEvent {
    /// Transition de phase.
    PhaseChanged { from: PlaybackState, to: PlaybackState },
    /// Progression de génération/lecture (segments consommés / total).
    Progress { current: usize, total: usize },
    /// Erreur terminale (message FR pour l'UI).
    Error { message: String },
}

type Listener = Arc<dyn Fn(StateEvent) + Send + Sync>;

/// Machine d'états de lecture.
pub struct PlaybackStateMachine {
    state: Arc<Mutex<PlaybackState>>,
    current_text: Arc<Mutex<String>>,
    current_chunk: Arc<Mutex<usize>>,
    total_chunks: Arc<Mutex<usize>>,
    error_message: Arc<Mutex<String>>,
    generation: Arc<AtomicU64>,
    cancel_requested: Arc<AtomicBool>,
    listeners: Arc<Mutex<Vec<Listener>>>,
}

impl Default for PlaybackStateMachine {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackState::Idle)),
            current_text: Arc::new(Mutex::new(String::new())),
            current_chunk: Arc::new(Mutex::new(0)),
            total_chunks: Arc::new(Mutex::new(0)),
            error_message: Arc::new(Mutex::new(String::new())),
            generation: Arc::new(AtomicU64::new(0)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl PlaybackStateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> PlaybackState {
        self.state.lock().unwrap().clone()
    }

    /// Enregistre un écouteur d'événements (appelé synchronement, donc non bloquant).
    pub fn subscribe(&self, listener: impl Fn(StateEvent) + Send + Sync + 'static) {
        self.listeners.lock().unwrap().push(Arc::new(listener));
    }

    /// Nombre d'écouteurs enregistrés (diagnostic/test).
    pub fn listener_count(&self) -> usize {
        self.listeners.lock().unwrap().len()
    }

    fn emit(&self, event: StateEvent) {
        let listeners = self.listeners.lock().unwrap().clone();
        for listener in listeners {
            listener(event.clone());
        }
    }

    /// Change de phase en émettant `PhaseChanged` (si la phase change réellement).
    fn set_phase(&self, to: PlaybackState) {
        let from = {
            let mut state = self.state.lock().unwrap();
            let from = *state;
            *state = to;
            from
        };
        if from != to {
            self.emit(StateEvent::PhaseChanged { from, to });
        }
    }

    pub fn set_text(&self, text: String, chunk_count: usize) {
        *self.current_text.lock().unwrap() = text;
        *self.total_chunks.lock().unwrap() = chunk_count;
        *self.current_chunk.lock().unwrap() = 0;
        self.emit(StateEvent::Progress { current: 0, total: chunk_count });
    }

    /// Idle/Stopped/Error → Capturing.
    pub fn begin_capture(&self) -> Result<(), String> {
        let current = *self.state.lock().unwrap();
        match current {
            PlaybackState::Idle | PlaybackState::Stopped | PlaybackState::Error => {
                self.set_phase(PlaybackState::Capturing);
                Ok(())
            }
            _ => Err(format!("capture impossible dans l'état {current}")),
        }
    }

    /// Capturing → Generating (la capture a réussi, la génération peut commencer).
    pub fn begin_generation_phase(&self) -> Result<(), String> {
        let current = *self.state.lock().unwrap();
        match current {
            PlaybackState::Capturing => {
                self.set_phase(PlaybackState::Generating);
                Ok(())
            }
            _ => Err(format!("génération impossible dans l'état {current}")),
        }
    }

    /// Generating/Idle/Stopped/Paused → Playing (premier segment en lecture).
    pub fn play(&self) -> Result<(), String> {
        let current = *self.state.lock().unwrap();
        match current {
            PlaybackState::Generating | PlaybackState::Idle | PlaybackState::Stopped | PlaybackState::Paused => {
                self.set_phase(PlaybackState::Playing);
                Ok(())
            }
            PlaybackState::Playing => Err("Déjà en lecture".to_string()),
            PlaybackState::Error => Err("État d'erreur".to_string()),
            PlaybackState::Capturing => Err("Capture en cours".to_string()),
        }
    }

    pub fn pause(&self) -> Result<(), String> {
        let current = *self.state.lock().unwrap();
        match current {
            PlaybackState::Playing => {
                self.set_phase(PlaybackState::Paused);
                Ok(())
            }
            _ => Err("Pas en lecture".to_string()),
        }
    }

    pub fn resume(&self) -> Result<(), String> {
        let current = *self.state.lock().unwrap();
        match current {
            PlaybackState::Paused => {
                self.set_phase(PlaybackState::Playing);
                Ok(())
            }
            _ => Err("Pas en pause".to_string()),
        }
    }

    /// Fin de lecture : passe à `Idle`, remet la progression à zéro, émet les événements.
    /// Appelé automatiquement par la boucle de contrôle quand le sink est vide et la phase active.
    pub fn set_idle(&self) {
        *self.current_chunk.lock().unwrap() = 0;
        *self.total_chunks.lock().unwrap() = 0;
        *self.current_text.lock().unwrap() = String::new();
        self.set_phase(PlaybackState::Idle);
        self.emit(StateEvent::Progress { current: 0, total: 0 });
    }

    /// Arrêt : Stopped, position remise à zéro, génération annulée.
    pub fn stop(&self) {
        self.set_phase(PlaybackState::Stopped);
        *self.current_chunk.lock().unwrap() = 0;
        self.request_cancel_generation();
        let total = *self.total_chunks.lock().unwrap();
        self.emit(StateEvent::Progress { current: 0, total });
    }

    pub fn next_chunk(&self) -> Option<usize> {
        let chunk = {
            let mut current = self.current_chunk.lock().unwrap();
            let total = *self.total_chunks.lock().unwrap();
            if *current < total {
                let chunk = *current;
                *current += 1;
                Some(chunk)
            } else {
                None
            }
        };
        if let Some(chunk) = chunk {
            let total = *self.total_chunks.lock().unwrap();
            self.emit(StateEvent::Progress { current: chunk + 1, total });
        }
        chunk
    }

    pub fn progress(&self) -> (usize, usize) {
        (*self.current_chunk.lock().unwrap(), *self.total_chunks.lock().unwrap())
    }

    /// Passe en `Error` (message FR pour l'UI) et émet les événements.
    pub fn set_error(&self, error: String) {
        *self.error_message.lock().unwrap() = error.clone();
        self.emit(StateEvent::Error { message: error });
        self.set_phase(PlaybackState::Error);
    }

    pub fn error_message(&self) -> String {
        self.error_message.lock().unwrap().clone()
    }

    /// Démarre (ou redémarre) un cycle de génération et retourne son jeton.
    pub fn begin_generation(&self) -> u64 {
        self.cancel_requested.store(false, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Demande l'annulation de la génération en cours.
    pub fn request_cancel_generation(&self) {
        self.cancel_requested.store(true, Ordering::SeqCst);
    }

    /// Retourne vrai si la génération `id` ne doit plus écrire dans la file.
    pub fn is_generation_cancelled(&self, id: u64) -> bool {
        id != self.generation.load(Ordering::SeqCst) || self.cancel_requested.load(Ordering::SeqCst)
    }
}

impl Clone for PlaybackStateMachine {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            current_text: self.current_text.clone(),
            current_chunk: self.current_chunk.clone(),
            total_chunks: self.total_chunks.clone(),
            error_message: self.error_message.clone(),
            generation: self.generation.clone(),
            cancel_requested: self.cancel_requested.clone(),
            listeners: self.listeners.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    #[test]
    fn test_state_machine() {
        let sm = PlaybackStateMachine::new();
        assert_eq!(sm.state(), PlaybackState::Idle);

        sm.play().unwrap();
        assert_eq!(sm.state(), PlaybackState::Playing);

        sm.pause().unwrap();
        assert_eq!(sm.state(), PlaybackState::Paused);

        sm.resume().unwrap();
        assert_eq!(sm.state(), PlaybackState::Playing);

        sm.stop();
        assert_eq!(sm.state(), PlaybackState::Stopped);
    }

    #[test]
    fn pause_hors_lecture_est_refusee() {
        let sm = PlaybackStateMachine::new();
        assert!(sm.pause().is_err());
        assert!(sm.resume().is_err());
    }

    #[test]
    fn stop_remet_la_position_a_zero() {
        let sm = PlaybackStateMachine::new();
        sm.set_text("bonjour".to_string(), 3);
        assert_eq!(sm.next_chunk(), Some(0));
        assert_eq!(sm.next_chunk(), Some(1));
        assert_eq!(sm.progress(), (2, 3));
        sm.stop();
        assert_eq!(sm.progress(), (0, 3));
        assert_eq!(sm.next_chunk(), Some(0));
    }

    #[test]
    fn stop_annule_la_generation_en_cours() {
        let sm = PlaybackStateMachine::new();
        let id = sm.begin_generation();
        assert!(!sm.is_generation_cancelled(id));
        sm.stop();
        assert!(sm.is_generation_cancelled(id));
    }

    #[test]
    fn begin_generation_invalide_le_jeton_precedent() {
        let sm = PlaybackStateMachine::new();
        let ancien = sm.begin_generation();
        let nouveau = sm.begin_generation();
        assert!(sm.is_generation_cancelled(ancien));
        assert!(!sm.is_generation_cancelled(nouveau));
    }

    #[test]
    fn error_est_terminal_sans_reset_explicite() {
        let sm = PlaybackStateMachine::new();
        sm.set_error("modèle absent".to_string());
        assert_eq!(sm.state(), PlaybackState::Error);
        assert!(sm.play().is_err());
        assert_eq!(sm.error_message(), "modèle absent");
    }

    #[test]
    fn resume_sans_pause_est_refusee() {
        let sm = PlaybackStateMachine::new();
        sm.play().unwrap();
        assert!(sm.resume().is_err());
    }

    #[test]
    fn relance_pendant_lecture_est_refusee_mais_stable() {
        let sm = PlaybackStateMachine::new();
        sm.set_text("t".to_string(), 2);
        sm.play().unwrap();
        assert!(sm.play().is_err());
        assert_eq!(sm.state(), PlaybackState::Playing);
        assert_eq!(sm.progress(), (0, 2));
    }

    #[test]
    fn pause_reprise_alternees_100_fois() {
        let sm = PlaybackStateMachine::new();
        sm.play().unwrap();
        for _ in 0..100 {
            sm.pause().unwrap();
            sm.resume().unwrap();
        }
        assert_eq!(sm.state(), PlaybackState::Playing);
    }

    #[test]
    fn stop_pendant_generation_annule_le_jeton() {
        let sm = PlaybackStateMachine::new();
        let generateur = sm.begin_generation();
        sm.stop();
        assert!(sm.is_generation_cancelled(generateur));
        sm.play().unwrap();
        let nouveau = sm.begin_generation();
        assert_ne!(generateur, nouveau);
        assert!(!sm.is_generation_cancelled(nouveau));
        assert!(sm.is_generation_cancelled(generateur));
    }

    #[test]
    fn le_stop_rapide_en_boucle_reste_coherent() {
        let sm = PlaybackStateMachine::new();
        sm.set_text("texte".to_string(), 4);
        for _ in 0..100 {
            let id = sm.begin_generation();
            sm.play().ok();
            sm.next_chunk();
            sm.stop();
            assert_eq!(sm.state(), PlaybackState::Stopped);
            assert_eq!(sm.progress(), (0, 4));
            assert!(sm.is_generation_cancelled(id));
        }
    }

    #[test]
    fn pause_apres_stop_est_refusee() {
        let sm = PlaybackStateMachine::new();
        sm.play().unwrap();
        sm.stop();
        assert!(sm.pause().is_err());
        sm.play().unwrap();
        assert_eq!(sm.state(), PlaybackState::Playing);
    }

    // --- Carte TW-032 : états de capture/génération et événements UI ---

    #[test]
    fn parcours_complet_capture_generation_lecture_pause_arret() {
        let sm = PlaybackStateMachine::new();
        sm.begin_capture().unwrap();
        assert_eq!(sm.state(), PlaybackState::Capturing);
        sm.begin_generation_phase().unwrap();
        assert_eq!(sm.state(), PlaybackState::Generating);
        sm.set_text("t".to_string(), 1);
        sm.play().unwrap();
        assert_eq!(sm.state(), PlaybackState::Playing);
        sm.pause().unwrap();
        sm.resume().unwrap();
        sm.stop();
        assert_eq!(sm.state(), PlaybackState::Stopped);
    }

    #[test]
    fn capture_refusee_si_pas_au_repos() {
        let sm = PlaybackStateMachine::new();
        sm.begin_capture().unwrap();
        assert!(sm.begin_capture().is_err());
    }

    #[test]
    fn les_ecouteurs_recoivent_les_transitions() {
        let sm = PlaybackStateMachine::new();
        let events = Arc::new(StdMutex::new(Vec::new()));
        let events2 = events.clone();
        sm.subscribe(move |evt| events2.lock().unwrap().push(evt));

        sm.begin_capture().unwrap();
        sm.begin_generation_phase().unwrap();
        sm.play().unwrap();
        sm.pause().unwrap();
        sm.stop();

        let captured = events.lock().unwrap();
        let phases: Vec<PlaybackState> = captured
            .iter()
            .filter_map(|e| match e {
                StateEvent::PhaseChanged { to, .. } => Some(*to),
                _ => None,
            })
            .collect();
        assert_eq!(
            phases,
            vec![
                PlaybackState::Capturing,
                PlaybackState::Generating,
                PlaybackState::Playing,
                PlaybackState::Paused,
                PlaybackState::Stopped,
            ]
        );
    }

    #[test]
    fn set_error_emet_evenement_error_puis_phase() {
        let sm = PlaybackStateMachine::new();
        let events = Arc::new(StdMutex::new(Vec::new()));
        let events2 = events.clone();
        sm.subscribe(move |evt| events2.lock().unwrap().push(evt));
        sm.set_error("modèle absent".to_string());
        let captured = events.lock().unwrap();
        assert!(captured.iter().any(|e| matches!(e, StateEvent::Error { message } if message == "modèle absent")));
        assert!(captured.iter().any(|e| matches!(e, StateEvent::PhaseChanged { to, .. } if *to == PlaybackState::Error)));
    }

    #[test]
    fn les_evenements_se_serialisent_en_json_stable() {
        let evt = StateEvent::PhaseChanged { from: PlaybackState::Idle, to: PlaybackState::Playing };
        let json = serde_json::to_string(&evt).unwrap();
        assert_eq!(json, r#"{"type":"phase-changed","from":"idle","to":"playing"}"#);

        let evt = StateEvent::Error { message: "erreur x".to_string() };
        let json = serde_json::to_string(&evt).unwrap();
        assert_eq!(json, r#"{"type":"error","message":"erreur x"}"#);
    }
}
