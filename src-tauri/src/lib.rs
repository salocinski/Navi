//! Navi - Lecteur de texte à voix haute 100% local
//! 
//! Architecture modulaire :
//! - config: gestion de configuration persistante
//! - capture: capture presse-papiers et texte sélectionné
//! - hotkey: raccourcis globaux système
//! - tts: moteur TTS (Kokoro via ONNX Runtime)
//! - audio: lecture audio (rodio)
//! - state: machine d'états de lecture
//! - ui: zone de notification, panneau, menu
//! - utils: utilitaires communs

pub mod config;
pub mod capture;
pub mod hotkey;
pub mod tts;
pub mod audio;
pub mod state;
pub mod ui;
pub mod utils;

use serde::Serialize;
use tauri::{Emitter, Manager};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// DTO de statut exposé à l'UI (lecture seule).
#[derive(Serialize)]
struct StatusDto {
    phase: crate::state::PlaybackState,
    current_chunk: usize,
    total_chunks: usize,
}

#[tauri::command]
fn get_status(state: tauri::State<crate::state::PlaybackStateMachine>) -> StatusDto {
    let (current_chunk, total_chunks) = state.progress();
    StatusDto { phase: state.state(), current_chunk, total_chunks }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_status,
        ])
        .setup(|app| {
            let machine = crate::state::PlaybackStateMachine::new();
            let handle = app.handle().clone();
            machine.subscribe(move |event| {
                // Événements consommés par l'UI via @tauri-apps/api/event (`navi-state`).
                let _ = handle.emit("navi-state", event);
            });
            app.manage(machine);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}