//! Installation des modèles de lecture Kokoro (carte N-066) : téléchargement vérifié au premier usage.
//!
//! Le vocabulaire (1 Ko) est embarqué dans l'application ; seuls le modèle et les voix sont téléchargés,
//! une fois, depuis la release GitHub `kokoro-onnx`. Ensuite la lecture fonctionne hors ligne.
//! Après une mise à jour (N-098), les fichiers attendus viennent du registre `models_store`.

use super::{KOKORO_FILES, KOKORO_MODEL_FILE, KOKORO_VOCAB_FILE, KOKORO_VOICES_FILE};
use crate::models_store::FileSpec;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const RELEASE: &str =
    "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0";
/// Fichiers téléchargés et leur taille attendue (l'empreinte vient de `KOKORO_FILES`).
const DOWNLOADS: [(&str, u64); 2] = [
    (KOKORO_MODEL_FILE, 92_361_271),
    (KOKORO_VOICES_FILE, 28_214_398),
];
/// Extrait de `config.json` de Kokoro-82M (Apache-2.0), octets identiques à l'empreinte attendue.
const VOCAB: &[u8] = include_bytes!("../../resources/kokoro-vocab.json");

// shortcut: progression partagée par des atomiques que la page Paramètres interroge ; passer à un
// événement Tauri si une interface plus réactive est nécessaire.
static DOWNLOADING: AtomicBool = AtomicBool::new(false);
static DONE: AtomicU64 = AtomicU64::new(0);
static TOTAL: AtomicU64 = AtomicU64::new(0);

fn sha256_of(name: &str) -> &'static str {
    KOKORO_FILES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, sha)| *sha)
        .expect("empreinte connue pour chaque fichier Kokoro")
}

/// Fichiers livrés avec cette version de Navi.
pub fn pinned_files() -> Vec<FileSpec> {
    DOWNLOADS
        .iter()
        .map(|(name, size)| FileSpec {
            name: (*name).into(),
            url: format!("{RELEASE}/{name}"),
            size: *size,
            sha256: sha256_of(name).into(),
        })
        .collect()
}

/// Fichiers attendus : ceux d'une mise à jour installée (N-098), sinon les fichiers livrés.
pub fn specs() -> Vec<FileSpec> {
    crate::models_store::files("kokoro")
}

/// Fichiers à télécharger : absents ou de taille inattendue (contrôle bon marché, sans hacher).
fn missing_in<'a>(dir: &Path, specs: &'a [FileSpec]) -> Vec<&'a FileSpec> {
    specs
        .iter()
        .filter(|s| {
            !std::fs::metadata(dir.join(&s.name)).is_ok_and(|m| m.is_file() && m.len() == s.size)
        })
        .collect()
}

pub fn is_installed(dir: &Path) -> bool {
    missing_in(dir, &specs()).is_empty()
}

/// (téléchargement en cours, octets reçus, octets attendus)
pub fn progress() -> (bool, u64, u64) {
    (
        DOWNLOADING.load(Ordering::SeqCst),
        DONE.load(Ordering::Relaxed),
        TOTAL.load(Ordering::Relaxed),
    )
}

fn ensure_vocab(dir: &Path) -> Result<(), String> {
    let path = dir.join(KOKORO_VOCAB_FILE);
    if std::fs::read(&path).is_ok_and(|bytes| bytes == VOCAB) {
        return Ok(());
    }
    std::fs::write(&path, VOCAB).map_err(|e| format!("{} : {e}", path.display()))
}

/// Installe les fichiers manquants dans `dir` (bloquant). Un seul téléchargement à la fois.
pub fn install(dir: &Path) -> Result<(), String> {
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err("Un téléchargement de modèle est déjà en cours".into());
    }
    let result = install_files(dir);
    DOWNLOADING.store(false, Ordering::SeqCst);
    result
}

fn install_files(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    ensure_vocab(dir)?;
    let specs = specs();
    let todo = missing_in(dir, &specs);
    TOTAL.store(todo.iter().map(|s| s.size).sum(), Ordering::Relaxed);
    DONE.store(0, Ordering::Relaxed);
    let mut base = 0;
    for spec in todo {
        crate::stt::model::download_to(
            &spec.url,
            &dir.join(&spec.name),
            spec.size,
            &spec.sha256,
            |done, _| DONE.store(base + done, Ordering::Relaxed),
        )?;
        base += spec.size;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn vocabulaire_embarque_conforme_a_l_empreinte() {
        let sum = format!("{:x}", Sha256::digest(VOCAB));
        assert_eq!(sum, sha256_of(KOKORO_VOCAB_FILE));
    }

    #[test]
    fn fichiers_manquants_ou_de_mauvaise_taille_detectes() {
        let dir = tempfile::tempdir().unwrap();
        let specs = pinned_files();
        assert_eq!(missing_in(dir.path(), &specs).len(), 2);
        std::fs::write(dir.path().join(KOKORO_MODEL_FILE), b"trop court").unwrap();
        assert_eq!(
            missing_in(dir.path(), &specs).len(),
            2,
            "taille inattendue = à retélécharger"
        );
    }

    #[test]
    fn vocabulaire_ecrit_puis_laisse_intact() {
        let dir = tempfile::tempdir().unwrap();
        ensure_vocab(dir.path()).unwrap();
        let path = dir.path().join(KOKORO_VOCAB_FILE);
        assert_eq!(std::fs::read(&path).unwrap(), VOCAB);
        std::fs::write(&path, b"{}").unwrap(); // fichier altéré : réécrit
        ensure_vocab(dir.path()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), VOCAB);
    }
}
