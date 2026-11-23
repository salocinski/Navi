//! Utilitaires communs

use std::path::PathBuf;

/// Variable d'environnement de développement : ignorée en release. Navi s'exécute élevé (N-095) ;
/// une variable posée par un processus non élevé ne doit pas rediriger un chemin de l'application.
pub fn dev_env(name: &str) -> Option<String> {
    if cfg!(debug_assertions) {
        std::env::var(name).ok()
    } else {
        None
    }
}

/// Vrai si une bibliothèque native (DLL) peut être chargée depuis `dir` : en release, uniquement le
/// dossier de l'application (protégé en écriture), jamais un dossier modifiable par l'utilisateur.
pub fn may_load_code_from(dir: &std::path::Path) -> bool {
    cfg!(debug_assertions)
        || std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| dir.starts_with(p)))
            .unwrap_or(false)
}

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
        if let Some(path) = dev_env(var) {
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
/// `<repo>/models/` → `<dossier de l'exe>/models/kokoro` (installé) → `%APPDATA%/Navi/models`.
pub fn resolve_models_dir() -> PathBuf {
    resolve_models_dir_with(None)
}

/// Comme `resolve_models_dir`, avec le dossier choisi dans les réglages (`models_dir`),
/// prioritaire sur les emplacements par défaut mais pas sur `NAVI_MODELS_DIR`.
pub fn resolve_models_dir_with(configured: Option<&std::path::Path>) -> PathBuf {
    if let Some(dir) = dev_env("NAVI_MODELS_DIR") {
        let path = PathBuf::from(&dir);
        if path.join("kokoro-v1.0.int8.onnx").exists() {
            return path;
        }
        if path.join("kokoro").join("kokoro-v1.0.int8.onnx").exists() {
            return path.join("kokoro");
        }
    }

    if let Some(dir) = configured {
        return dir.to_path_buf();
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

    // Application installée (carte N-060) : modèles embarqués à côté de l'exécutable.
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        let candidate = exe_dir.join("models").join("kokoro");
        if candidate.join("kokoro-v1.0.int8.onnx").exists() {
            return candidate;
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
