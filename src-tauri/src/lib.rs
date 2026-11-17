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

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            // Seront ajoutés : commandes capture, hotkey, tts, audio, state, ui
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}