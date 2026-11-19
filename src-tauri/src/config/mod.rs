//! Gestion de configuration persistante

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Langue par défaut pour le TTS
    pub default_language: String,

    /// Voix féminine (ID Kokoro)
    pub voice_female: String,

    /// Voix masculine (ID Kokoro)
    pub voice_male: String,

    /// Vitesse de lecture (0.5 - 2.0)
    pub speech_rate: f32,

    /// Volume global (0.0 - 1.0)
    pub volume: f32,

    /// Raccourci lecture (ex: "Ctrl+Shift+R")
    pub hotkey_read: String,

    /// Raccourci pause/reprise
    pub hotkey_pause: String,

    /// Raccourci arrêt
    pub hotkey_stop: String,

    /// Lancer au démarrage Windows
    pub autostart: bool,

    /// Répertoire des modèles (optionnel, défaut: ~/AppData/Navi/models)
    pub models_dir: Option<std::path::PathBuf>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_language: "auto".to_string(),
            voice_female: "af_heart".to_string(),
            voice_male: "am_adam".to_string(),
            speech_rate: 1.0,
            volume: 0.8,
            hotkey_read: "Ctrl+Shift+R".to_string(),
            hotkey_pause: "Ctrl+Shift+P".to_string(),
            hotkey_stop: "Ctrl+Shift+S".to_string(),
            autostart: false,
            models_dir: None,
        }
    }
}

impl AppConfig {
    /// Charge la configuration depuis le fichier
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Sauvegarde la configuration
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        Ok(dirs::data_dir()
            .ok_or("Impossible de déterminer le répertoire de données")?
            .join("Navi")
            .join("config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.speech_rate, 1.0);
        assert_eq!(config.volume, 0.8);
        assert_eq!(config.voice_female, "af_heart");
        assert_eq!(config.voice_male, "am_adam");
    }
}
