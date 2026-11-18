//! Machine d'états de lecture

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
    Stopped,
    Error,
}

/// Machine d'états de lecture.
///
/// Porte aussi l'annulation de la génération en cours : un arrêt
/// invalide le jeton de génération courant, un worker TTS qui vérifie
/// `is_generation_cancelled(id)` entre deux segments doit abandonner
/// et libérer ses ressources sans écrire dans la file audio.
pub struct PlaybackStateMachine {
    state: Arc<Mutex<PlaybackState>>,
    current_text: Arc<Mutex<String>>,
    current_chunk: Arc<Mutex<usize>>,
    total_chunks: Arc<Mutex<usize>>,
    error_message: Arc<Mutex<String>>,
    generation: Arc<AtomicU64>,
    cancel_requested: Arc<AtomicBool>,
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

    pub fn set_text(&self, text: String, chunk_count: usize) {
        *self.current_text.lock().unwrap() = text;
        *self.total_chunks.lock().unwrap() = chunk_count;
        *self.current_chunk.lock().unwrap() = 0;
    }

    pub fn play(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        match *state {
            PlaybackState::Idle | PlaybackState::Stopped | PlaybackState::Paused => {
                *state = PlaybackState::Playing;
                Ok(())
            }
            PlaybackState::Playing => Err("Déjà en lecture".to_string()),
            PlaybackState::Error => Err("État d'erreur".to_string()),
        }
    }

    pub fn pause(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        match *state {
            PlaybackState::Playing => {
                *state = PlaybackState::Paused;
                Ok(())
            }
            _ => Err("Pas en lecture".to_string()),
        }
    }

    pub fn resume(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        match *state {
            PlaybackState::Paused => {
                *state = PlaybackState::Playing;
                Ok(())
            }
            _ => Err("Pas en pause".to_string()),
        }
    }

    /// Arrête la lecture : passe à `Stopped`, remet la position à zéro
    /// et annule la génération en cours.
    pub fn stop(&self) {
        *self.state.lock().unwrap() = PlaybackState::Stopped;
        *self.current_chunk.lock().unwrap() = 0;
        self.request_cancel_generation();
    }

    pub fn next_chunk(&self) -> Option<usize> {
        let mut current = self.current_chunk.lock().unwrap();
        let total = *self.total_chunks.lock().unwrap();
        if *current < total {
            let chunk = *current;
            *current += 1;
            Some(chunk)
        } else {
            None
        }
    }

    pub fn progress(&self) -> (usize, usize) {
        (*self.current_chunk.lock().unwrap(), *self.total_chunks.lock().unwrap())
    }

    pub fn set_error(&self, error: String) {
        *self.state.lock().unwrap() = PlaybackState::Error;
        *self.error_message.lock().unwrap() = error;
    }

    pub fn error_message(&self) -> String {
        self.error_message.lock().unwrap().clone()
    }

    /// Démarre (ou redémarre) un cycle de génération et retourne son jeton.
    /// Chaque appel invalide le jeton précédent.
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
