//! Machine d'états de lecture

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
    Stopped,
    Error,
}

/// Machine d'états de lecture
pub struct PlaybackStateMachine {
    state: Arc<Mutex<PlaybackState>>,
    current_text: Arc<Mutex<String>>,
    current_chunk: Arc<Mutex<usize>>,
    total_chunks: Arc<Mutex<usize>>,
    error_message: Arc<Mutex<String>>,
}

impl Default for PlaybackStateMachine {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackState::Idle)),
            current_text: Arc::new(Mutex::new(String::new())),
            current_chunk: Arc::new(Mutex::new(0)),
            total_chunks: Arc::new(Mutex::new(0)),
            error_message: Arc::new(Mutex::new(String::new())),
        }
    }
}

impl PlaybackStateMachine {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn state(&self) -> PlaybackState {
        *self.state.lock().unwrap()
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
    
    pub fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        *state = PlaybackState::Stopped;
        *self.current_chunk.lock().unwrap() = 0;
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
}

impl Clone for PlaybackStateMachine {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            current_text: self.current_text.clone(),
            current_chunk: self.current_chunk.clone(),
            total_chunks: self.total_chunks.clone(),
            error_message: self.error_message.clone(),
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
}