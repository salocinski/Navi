//! Navi - Lecteur de texte à voix haute 100% local
//! 
//! Architecture modulaire :
//! - config: gestion de configuration persistante
//! - capture: capture presse-papiers et texte sélectionné
//! - hotkey: raccourcis globaux système
//! - tts: moteur TTS (Kokoro via ONNX Runtime)
//! - audio: lecture audio (rodio)
//! - state: machine d'états de lecture
//! - text: nettoyage, langue, segmentation en production
//! - pipeline: chaîne lecture (capture → moteur → audio)
//! - ui: zone de notification, panneau, menu
//! - utils: utilitaires communs

pub mod config;
pub mod capture;
pub mod hotkey;
pub mod tts;
pub mod audio;
pub mod state;
pub mod text;
pub mod pipeline;
pub mod ui;
pub mod utils;

use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// État partagé de Navi (machines à états + moteur Kokoro + audio).
pub struct NaviState {
    pub machine: crate::state::PlaybackStateMachine,
    pub engine: Arc<Mutex<Option<crate::tts::KokoroEngine>>>,
    pub audio: Arc<Mutex<Option<crate::audio::AudioHandle>>>,
    /// Garde anti double-chargement (ort n'autorise qu'un seul `Environment` par processus).
    pub loading: Arc<std::sync::atomic::AtomicBool>,
}

impl NaviState {
    fn new() -> Self {
        Self {
            machine: crate::state::PlaybackStateMachine::new(),
            engine: Arc::new(Mutex::new(None)),
            audio: Arc::new(Mutex::new(None)),
            loading: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// DTO de statut exposé à l'UI (lecture seule).
#[derive(Serialize)]
struct StatusDto {
    phase: crate::state::PlaybackState,
    current_chunk: usize,
    total_chunks: usize,
    engine_loaded: bool,
    audio_ready: bool,
}

#[tauri::command]
fn get_status(state: tauri::State<NaviState>) -> StatusDto {
    let (current_chunk, total_chunks) = state.machine.progress();
    StatusDto {
        phase: state.machine.state(),
        current_chunk,
        total_chunks,
        engine_loaded: state.engine.lock().map(|e| e.is_some()).unwrap_or(false),
        audio_ready: state.audio.lock().map(|a| a.is_some()).unwrap_or(false),
    }
}

/// Charge le modèle Kokoro en tâche de fond (idempotent ; ~11 s au premier appel).
/// Un seul chargement simultané est autorisé — ort refuse plusieurs `Environment`.
#[tauri::command]
fn load_model(state: tauri::State<NaviState>) -> Result<(), String> {
    if state.engine.lock().map_err(|_| "verrou moteur".to_string())?.is_some() {
        return Ok(());
    }
    if state.loading.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Ok(()); // un chargement est déjà en cours
    }
    let engine = state.engine.clone();
    let loading = state.loading.clone();
    let machine = state.machine.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<crate::tts::KokoroEngine, String> {
            let dir = crate::utils::resolve_models_dir();
            let config = crate::tts::TtsConfig::with_dir(&dir);
            let mut kokoro = crate::tts::KokoroEngine::new(config)
                .map_err(|e| format!("chargement moteur impossible ({e}). Modèles attendus dans {}", dir.display()))?;
            crate::tts::TtsEngine::load(&mut kokoro)
                .map_err(|e| format!("modèle Kokoro non chargeable : {e}"))?;
            Ok(kokoro)
        })();
        match result {
            Ok(kokoro) => {
                if let Ok(mut guard) = engine.lock() {
                    *guard = Some(kokoro);
                }
            }
            Err(message) => {
                eprintln!("[navi] chargement moteur échoué : {message}");
                machine.set_error(message);
            }
        }
        loading.store(false, std::sync::atomic::Ordering::SeqCst);
    });
    Ok(())
}

/// Lance la lecture de la sélection courante : capture → nettoyage → langue → segments → audio.
#[tauri::command]
fn start_reading(state: tauri::State<NaviState>) -> Result<String, String> {
    state.machine.begin_capture().map_err(|e| e)?;

    // 1. Capture (snapshot presse-papiers + SendInput Ctrl+C + restauration)
    let cap = crate::capture::capture_selection();
    if !cap.success {
        let message = cap.error.clone().unwrap_or_else(|| "Capture échouée".to_string());
        state.machine.set_error(message.clone());
        return Err(message);
    }

    // 2. Nettoyage (HTML/RTF/ANSI/Unicode — médias/markup retirés, erreur claire sinon)
    let format_hint = cap.format_used.to_lowercase();
    let cleaned = match crate::text::clean_captured_text(&cap.text, &format_hint) {
        Ok(text) => text,
        Err(err) => {
            let message = err.to_string();
            state.machine.set_error(message.clone());
            return Err(message);
        }
    };

    // 3. Langue (détection hors moteur ; hint fr en cas de doute)
    let language = Some(match crate::text::detect_language(&cleaned) {
        crate::text::LanguageDetection::English => crate::tts::engine::TtsLanguage::English,
        _ => crate::tts::engine::TtsLanguage::French,
    });

    // 4. Segments
    let segments = crate::text::segmentation::segment_text(&cleaned);
    if segments.is_empty() {
        let message = "Aucun segment à lire".to_string();
        state.machine.set_error(message.clone());
        return Err(message);
    }

    let segment_count = segments.len();
    state.machine.begin_generation_phase().map_err(|e| e)?;
    state.machine.set_text(cleaned, segment_count);

    // 5. Audio : reset (nouvelle session) ou création (thread audio) si absent
    {
        let mut audio = state.audio.lock().map_err(|_| "verrou audio".to_string())?;
        match audio.as_ref() {
            Some(handle) => handle.reset()?,
            None => {
                let handle = crate::audio::AudioHandle::new()
                    .map_err(|e| format!("Aucun périphérique audio par défaut : {e}"))?;
                *audio = Some(handle);
            }
        }
    }

    // 6. Modèle requis — sinon déclencher un chargement en tâche de fond et demander un retry
    if state.engine.lock().map_err(|_| "verrou moteur".to_string())?.is_none() {
        let engine = state.engine.clone();
        let machine = state.machine.clone();
        let loading = state.loading.clone();
        load_model_bg(engine, machine, loading);
        let message = "Modèle Kokoro en cours de chargement… relancez la lecture dans quelques secondes".to_string();
        state.machine.set_error(message.clone());
        return Err(message);
    }

    // 7. Jeton de génération + thread de synthèse
    let token = state.machine.begin_generation();
    state.machine.play().map_err(|e| e)?;

    let machine = state.machine.clone();
    let engine = state.engine.clone();
    let segments = segments;
    let audio = state.audio.clone();
    std::thread::spawn(move || {
        eprintln!("[navi] thread de synthèse démarré ({} segments)", segments.len());
        let outcome = (|| -> Result<usize, String> {
            let engine_guard = engine.lock().map_err(|_| "verrou moteur".to_string())?;
            let kokoro = engine_guard.as_ref().ok_or_else(|| "Modèle Kokoro non chargé".to_string())?;
            let sink = audio
                .lock()
                .map_err(|_| "verrou audio".to_string())?
                .as_ref()
                .cloned()
                .ok_or_else(|| "Périphérique audio disparu".to_string())?;
            crate::pipeline::synthesize_segments(
                kokoro as &dyn crate::tts::engine::TtsEngine,
                &segments,
                &sink,
                &|| machine.is_generation_cancelled(token),
                language,
                &|| {
                    let _ = machine.next_chunk();
                },
            )
        })();
        match outcome {
            Ok(done) => eprintln!("[navi] synthèse terminée — {done} segments envoyés dans la file audio"),
            Err(message) => {
                eprintln!("[navi] synthèse échouée : {message}");
                machine.set_error(message);
            }
        }
    });

    Ok(format!("Lecture en cours — {segment_count} segments"))
}

/// Variante sans accès à `State` — utilisée par `start_reading` pour démarrer un chargement.
/// Refuse de spawned si un chargement est déjà en cours (ort : un seul Environment).
fn load_model_bg(
    engine: Arc<Mutex<Option<crate::tts::KokoroEngine>>>,
    machine: crate::state::PlaybackStateMachine,
    loading: Arc<std::sync::atomic::AtomicBool>,
) {
    if engine.lock().map(|e| e.is_some()).unwrap_or(true) {
        return;
    }
    if loading.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return; // déjà en cours
    }
    std::thread::spawn(move || {
        let dir = crate::utils::resolve_models_dir();
        let result = (|| -> Result<crate::tts::KokoroEngine, String> {
            let config = crate::tts::TtsConfig::with_dir(&dir);
            let mut kokoro = crate::tts::KokoroEngine::new(config)
                .map_err(|e| format!("chargement moteur impossible ({e}). Modèles attendus dans {}", dir.display()))?;
            crate::tts::TtsEngine::load(&mut kokoro).map_err(|e| format!("modèle Kokoro non chargeable : {e}"))?;
            Ok(kokoro)
        })();
        match result {
            Ok(kokoro) => {
                if let Ok(mut guard) = engine.lock() {
                    *guard = Some(kokoro);
                }
            }
            Err(message) => machine.set_error(message),
        }
        loading.store(false, std::sync::atomic::Ordering::SeqCst);
    });
}

#[tauri::command]
fn pause_reading(state: tauri::State<NaviState>) -> Result<(), String> {
    state.machine.pause().map_err(|e| e)?;
    state
        .audio
        .lock()
        .map_err(|_| "verrou audio".to_string())?
        .as_ref()
        .ok_or_else(|| "Audio non prêt".to_string())?
        .pause()
}

#[tauri::command]
fn resume_reading(state: tauri::State<NaviState>) -> Result<(), String> {
    state.machine.resume().map_err(|e| e)?;
    state
        .audio
        .lock()
        .map_err(|_| "verrou audio".to_string())?
        .as_ref()
        .ok_or_else(|| "Audio non prêt".to_string())?
        .resume()
}

#[tauri::command]
fn stop_reading(state: tauri::State<NaviState>) -> Result<(), String> {
    state.machine.stop();
    if let Ok(audio) = state.audio.lock() {
        if let Some(handle) = audio.as_ref() {
            let _ = handle.stop();
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_status,
            load_model,
            start_reading,
            pause_reading,
            resume_reading,
            stop_reading,
        ])
        .setup(|app| {
            let state = NaviState::new();
            let handle = app.handle().clone();
            state.machine.subscribe(move |event| {
                // Événements consommés par l'UI via @tauri-apps/api/event (`navi-state`).
                let _ = handle.emit("navi-state", event);
            });
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}