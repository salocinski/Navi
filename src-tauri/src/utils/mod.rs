//! Utilitaires communs

use std::path::PathBuf;

/// Retourne le chemin du répertoire de données de l'application
pub fn app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Navi"))
}

/// Nom du fichier de signature de démarrage (enregistrement fourni par le
/// propriétaire, carte N-083). Remplaçait la signature générée `hey_listen.wav`.
const SIGNATURE_FILE: &str = "hey_listen.mp3";

/// Chemin de la signature sonore de démarrage.
///
/// Ordre : `NAVI_SIGNATURE_FILE` → `NAVI_SIGNATURE_WAV` (compatibilité) →
/// `CARGO_MANIFEST_DIR/../resources/` (dév) → dossier `resources/` à côté de
/// l'exécutable / dans le bundle.
pub fn resolve_signature() -> PathBuf {
    for var in ["NAVI_SIGNATURE_FILE", "NAVI_SIGNATURE_WAV"] {
        if let Ok(path) = std::env::var(var) {
            let p = PathBuf::from(path);
            if p.exists() {
                return p;
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut bases = vec![
        manifest.join("..").join("resources"),
        manifest.join("resources"),
    ];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        bases.push(dir.join("resources"));
        bases.push(dir.join("..").join("resources"));
    }

    for base in bases {
        let candidate = base.join(SIGNATURE_FILE);
        if let Ok(normalized) = candidate.canonicalize() {
            return normalized;
        }
        if candidate.exists() {
            return candidate;
        }
    }
    manifest.join("resources").join(SIGNATURE_FILE)
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
        if let Ok(normalized) = candidate.canonicalize()
            && normalized.join("kokoro-v1.0.int8.onnx").exists()
        {
            return normalized;
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
