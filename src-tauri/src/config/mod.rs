//! Gestion de configuration persistante

use serde::{Deserialize, Serialize};

// `serde(default)` : un config.json écrit avant l'ajout d'un champ reste lisible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

    /// Jouer la signature sonore au démarrage (carte N-058)
    pub startup_sound: bool,

    /// Volume de la signature sonore (0.0 - 1.0), indépendant du volume de lecture
    pub startup_sound_volume: f32,

    /// Raccourci lecture (ex: "Super+Alt+S", carte N-072)
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
            volume: 1.0,
            startup_sound: true,
            startup_sound_volume: 1.0,
            hotkey_read: "Super+Alt+S".to_string(),
            hotkey_pause: "Ctrl+Shift+P".to_string(),
            hotkey_stop: "Ctrl+Shift+S".to_string(),
            autostart: false,
            models_dir: None,
        }
    }
}

impl AppConfig {
    /// Charge la configuration depuis le fichier (défauts si absent).
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_from(&Self::config_path()?)
    }

    /// Sauvegarde la configuration.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_to(&Self::config_path()?)
    }

    pub(crate) fn load_from(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Écriture atomique (fichier temporaire puis renommage) : un arrêt en cours
    /// d'écriture ne laisse jamais un `config.json` tronqué.
    pub(crate) fn save_to(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Valide une configuration reçue de l'UI (frontière de confiance).
    pub fn validate(&self) -> Result<(), String> {
        if !["auto", "fr", "en"].contains(&self.default_language.as_str()) {
            return Err(format!("Langue inconnue : « {} »", self.default_language));
        }
        if self.voice_female.trim().is_empty() || self.voice_male.trim().is_empty() {
            return Err("Les identifiants de voix ne peuvent pas être vides".into());
        }
        if !(0.5..=2.0).contains(&self.speech_rate) {
            return Err("La vitesse doit être comprise entre 0,5 et 2,0".into());
        }
        for (nom, v) in [
            ("Le volume", self.volume),
            ("Le volume de la signature", self.startup_sound_volume),
        ] {
            if !(0.0..=1.0).contains(&v) {
                return Err(format!("{nom} doit être compris entre 0 et 1"));
            }
        }
        for (nom, combo) in [
            ("lecture", &self.hotkey_read),
            ("pause/reprise", &self.hotkey_pause),
            ("arrêt", &self.hotkey_stop),
        ] {
            if combo.parse::<global_hotkey::hotkey::HotKey>().is_err() {
                return Err(format!("Raccourci {nom} invalide : « {combo} »"));
            }
        }
        if let Some(dir) = &self.models_dir
            && dir.as_os_str().is_empty()
        {
            return Err("Le dossier des modèles ne peut pas être vide (laisser non défini)".into());
        }
        Ok(())
    }

    /// Volume effectif de la signature : borné à 0.0–1.0, 0.0 si la valeur n'est pas un nombre.
    pub fn startup_sound_gain(&self) -> f32 {
        if self.startup_sound_volume.is_nan() {
            0.0
        } else {
            self.startup_sound_volume.clamp(0.0, 1.0)
        }
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
        assert_eq!(config.volume, 1.0);
        assert_eq!(config.voice_female, "af_heart");
        assert_eq!(config.voice_male, "am_adam");
    }

    #[test]
    fn signature_activee_a_plein_volume_par_defaut() {
        let c = AppConfig::default();
        assert!(c.startup_sound);
        assert_eq!(c.startup_sound_gain(), 1.0);
    }

    #[test]
    fn volume_signature_borne() {
        let gain = |v: f32| {
            AppConfig {
                startup_sound_volume: v,
                ..Default::default()
            }
            .startup_sound_gain()
        };
        assert_eq!(gain(7.0), 1.0);
        assert_eq!(gain(-1.0), 0.0);
        assert_eq!(gain(f32::NAN), 0.0);
    }

    #[test]
    fn ancien_config_json_reste_lisible() {
        let c: AppConfig = serde_json::from_str(r#"{"volume":0.5}"#).unwrap();
        assert_eq!(c.volume, 0.5);
        assert!(c.startup_sound);
    }

    #[test]
    fn config_par_defaut_valide() {
        AppConfig::default().validate().unwrap();
    }

    #[test]
    fn validation_refuse_les_valeurs_hors_bornes() {
        let ko = |c: AppConfig| assert!(c.validate().is_err());
        ko(AppConfig {
            default_language: "de".into(),
            ..Default::default()
        });
        ko(AppConfig {
            speech_rate: 3.0,
            ..Default::default()
        });
        ko(AppConfig {
            speech_rate: f32::NAN,
            ..Default::default()
        });
        ko(AppConfig {
            volume: 1.5,
            ..Default::default()
        });
        ko(AppConfig {
            startup_sound_volume: -0.1,
            ..Default::default()
        });
        ko(AppConfig {
            voice_male: "  ".into(),
            ..Default::default()
        });
        ko(AppConfig {
            hotkey_stop: "n'importe quoi".into(),
            ..Default::default()
        });
        ko(AppConfig {
            models_dir: Some("".into()),
            ..Default::default()
        });
    }

    #[test]
    fn sauvegarde_puis_relecture() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sous").join("config.json");
        let c = AppConfig {
            volume: 0.3,
            startup_sound: false,
            ..Default::default()
        };
        c.save_to(&path).unwrap();
        let lu = AppConfig::load_from(&path).unwrap();
        assert_eq!(lu.volume, 0.3);
        assert!(!lu.startup_sound);
        assert!(
            !path.with_extension("json.tmp").exists(),
            "pas de temporaire résiduel"
        );
    }

    #[test]
    fn fichier_absent_donne_les_defauts_et_corrompu_une_erreur() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        assert_eq!(AppConfig::load_from(&path).unwrap().volume, 1.0);
        std::fs::write(&path, "{ pas du json").unwrap();
        assert!(AppConfig::load_from(&path).is_err());
    }
}
