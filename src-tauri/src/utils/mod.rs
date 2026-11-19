//! Utilitaires communs

use std::path::PathBuf;

/// Retourne le chemin du répertoire de données de l'application
pub fn app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Navi"))
}

/// Chemin de la signature sonore de démarrage (`hey_listen.wav`).
///
/// Ordre : variable `NAVI_SIGNATURE_WAV` → `CARGO_MANIFEST_DIR/../resources/hey_listen.wav`
/// (dév) → dossier `resources/` à côté de l'exécutable / dans le bundle.
pub fn resolve_signature_wav() -> PathBuf {
    if let Ok(path) = std::env::var("NAVI_SIGNATURE_WAV") {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("..").join("resources").join("hey_listen.wav"),
        manifest.join("resources").join("hey_listen.wav"),
        std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.parent()
                    .map(|d| d.join("resources").join("hey_listen.wav"))
            })
            .unwrap_or_else(|| PathBuf::from("resources/hey_listen.wav")),
        std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.parent()
                    .map(|d| d.join("..").join("resources").join("hey_listen.wav"))
            })
            .unwrap_or_else(|| PathBuf::from("../resources/hey_listen.wav")),
    ];
    for candidate in candidates {
        if let Ok(p) = candidate.canonicalize() {
            if p.exists() {
                return p;
            }
        } else if candidate.exists() {
            return candidate;
        }
    }
    manifest.join("resources").join("hey_listen.wav")
}

/// Retourne le chemin du répertoire des modèles — dev ou utilisateur.
///
/// Ordre : `NAVI_MODELS_DIR` → `<repo>/models/kokoro/` (si l'ONNX y existe) →
/// `<repo>/models/` → `%APPDATA%/Navi/models`.
pub fn resolve_models_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("NAVI_MODELS_DIR") {
        let path = PathBuf::from(&dir);
        if path.join("kokoro-v1.0.int8.onnx").exists() {
            return path;
        }
        if path.join("kokoro").join("kokoro-v1.0.int8.onnx").exists() {
            return path.join("kokoro");
        }
    }

    // Racine du dépôt : le crate est `src-tauri`, les modèles sont à `models/kokoro/`.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for candidate in [
        manifest.join("..").join("models").join("kokoro"),
        manifest.join("models").join("kokoro"),
        manifest.join("..").join("models"),
    ] {
        if let Ok(normalized) = candidate.canonicalize() {
            if normalized.join("kokoro-v1.0.int8.onnx").exists() {
                return normalized;
            }
        }
    }

    // Repli cwd (lancer depuis la racine du projet)
    if let Ok(local) = std::env::current_dir() {
        for candidate in [local.join("models").join("kokoro"), local.join("models")] {
            if candidate.join("kokoro-v1.0.int8.onnx").exists() {
                return candidate;
            }
        }
    }

    app_data_dir()
        .map(|d| d.join("models"))
        .unwrap_or_else(|| PathBuf::from("./models"))
}

/// Retourne le chemin du répertoire de données de l'application
pub fn models_dir() -> Option<PathBuf> {
    Some(resolve_models_dir())
}

/// Nettoie et normalise un texte capturé (délègue à `text::normalize_ws`)
pub fn clean_text(text: &str) -> String {
    crate::text::normalize_ws(text)
}

/// Détecte la langue probable d'un texte (délègue à `text::detect_language`, retourne « fr »/« en »)
pub fn detect_language(text: &str) -> &'static str {
    crate::text::detect_language(text).as_tts_hint()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("  Hello\x00World\n  "), "HelloWorld");
        assert_eq!(clean_text("\t\tTest\n\n"), "Test");
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("Hello world this is a test"), "en");
        assert_eq!(detect_language("Bonjour le monde c'est un test"), "fr");
    }
}
