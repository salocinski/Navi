//! Registre des modèles installés et remplacement atomique (carte N-098).
//!
//! Les empreintes épinglées dans le code décrivent les modèles livrés avec cette version de Navi. Une mise à jour
//! vérifiée (manifeste signé, voir `models_update`) est enregistrée ici : le moteur consulte d'abord ce registre,
//! puis retombe sur les valeurs épinglées. La mise à jour est téléchargée en `<nom>.new` puis appliquée au
//! démarrage suivant, avant tout chargement du modèle (un fichier ouvert ne peut pas être remplacé sous Windows).
//! L'ancien fichier est gardé en `<nom>.previous` jusqu'au premier usage réussi du nouveau (`confirm_use`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Modèles gérés (identifiants du manifeste).
pub const IDS: [&str; 2] = ["kokoro", "whisper"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileSpec {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub version: String,
    pub files: Vec<FileSpec>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
struct Entry {
    /// `None` : version livrée avec l'application (valeurs épinglées).
    current: Option<Installed>,
    previous: Option<Installed>,
    /// Mise à jour téléchargée (`.new`), à appliquer au prochain démarrage.
    pending: Option<Installed>,
    /// Retour à `previous` demandé, à appliquer au prochain démarrage.
    rollback: bool,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Store {
    models: BTreeMap<String, Entry>,
}

impl Store {
    /// Fichiers attendus pour `id` : ceux du registre, sinon les valeurs épinglées.
    pub fn files(&self, id: &str, pinned: &Installed) -> Vec<FileSpec> {
        self.models
            .get(id)
            .and_then(|e| e.current.as_ref())
            .map_or_else(|| pinned.files.clone(), |i| i.files.clone())
    }

    pub fn version(&self, id: &str, pinned: &Installed) -> String {
        self.models
            .get(id)
            .and_then(|e| e.current.as_ref())
            .map_or_else(|| pinned.version.clone(), |i| i.version.clone())
    }

    pub fn has_previous(&self, id: &str) -> bool {
        self.models.get(id).is_some_and(|e| e.previous.is_some())
    }

    /// `"update"` ou `"rollback"` si un changement sera appliqué au prochain démarrage.
    pub fn pending(&self, id: &str) -> Option<&'static str> {
        let entry = self.models.get(id)?;
        if entry.pending.is_some() {
            Some("update")
        } else if entry.rollback {
            Some("rollback")
        } else {
            None
        }
    }

    pub fn stage_update(&mut self, id: &str, installed: Installed) {
        let entry = self.models.entry(id.to_string()).or_default();
        entry.pending = Some(installed);
        entry.rollback = false;
    }

    pub fn stage_rollback(&mut self, id: &str) -> Result<(), String> {
        let entry = self.models.entry(id.to_string()).or_default();
        if entry.previous.is_none() {
            return Err("Aucune version précédente conservée".into());
        }
        entry.pending = None;
        entry.rollback = true;
        Ok(())
    }
}

fn sibling(dir: &Path, name: &str, suffix: &str) -> PathBuf {
    dir.join(format!("{name}{suffix}"))
}

fn remove_staged(dir: &Path, files: &[FileSpec]) {
    for f in files {
        let _ = std::fs::remove_file(sibling(dir, &f.name, ".new"));
    }
}

/// Remet en place (au mieux) les fichiers déjà permutés : `.previous` redevient le fichier actif.
fn undo(dir: &Path, name: &str) {
    let (dest, new, prev) = (
        dir.join(name),
        sibling(dir, name, ".new"),
        sibling(dir, name, ".previous"),
    );
    if prev.exists() {
        if dest.exists() {
            let _ = std::fs::rename(&dest, new);
        }
        let _ = std::fs::rename(prev, dest);
    }
}

fn swap_in(dir: &Path, files: &[FileSpec]) -> Result<(), String> {
    for (i, f) in files.iter().enumerate() {
        let (dest, new, prev) = (
            dir.join(&f.name),
            sibling(dir, &f.name, ".new"),
            sibling(dir, &f.name, ".previous"),
        );
        let step = || -> std::io::Result<()> {
            let _ = std::fs::remove_file(&prev);
            if dest.exists() {
                std::fs::rename(&dest, &prev)?;
            }
            std::fs::rename(&new, &dest)
        };
        if let Err(e) = step() {
            for g in files[..=i].iter().rev() {
                undo(dir, &g.name);
            }
            return Err(format!("Remplacement de {} impossible : {e}", f.name));
        }
    }
    Ok(())
}

fn swap_back(dir: &Path, files: &[FileSpec]) -> Result<(), String> {
    for f in files {
        let (dest, prev, tmp) = (
            dir.join(&f.name),
            sibling(dir, &f.name, ".previous"),
            sibling(dir, &f.name, ".rollback"),
        );
        let step = || -> std::io::Result<()> {
            std::fs::rename(&dest, &tmp)?;
            if let Err(e) = std::fs::rename(&prev, &dest) {
                let _ = std::fs::rename(&tmp, &dest);
                return Err(e);
            }
            let _ = std::fs::remove_file(&tmp);
            Ok(())
        };
        step().map_err(|e| {
            format!(
                "Retour à la version précédente impossible ({}) : {e}",
                f.name
            )
        })?;
    }
    Ok(())
}

/// Applique la mise à jour ou le retour arrière en attente pour `id` ; vrai si le registre a changé.
/// Une mise à jour dont les fichiers `.new` manquent ou sont incomplets est abandonnée sans toucher au modèle actif.
pub fn apply_pending_in(
    store: &mut Store,
    id: &str,
    dir: &Path,
    pinned: &Installed,
) -> Result<bool, String> {
    let Some(entry) = store.models.get_mut(id) else {
        return Ok(false);
    };
    if let Some(new) = entry.pending.take() {
        let complete = new.files.iter().all(|f| {
            std::fs::metadata(sibling(dir, &f.name, ".new"))
                .is_ok_and(|m| m.is_file() && m.len() == f.size)
        });
        if !complete {
            remove_staged(dir, &new.files);
            return Err("Fichiers de mise à jour incomplets : mise à jour abandonnée".into());
        }
        if let Err(e) = swap_in(dir, &new.files) {
            remove_staged(dir, &new.files);
            return Err(e);
        }
        entry.previous = Some(entry.current.take().unwrap_or_else(|| pinned.clone()));
        entry.current = Some(new);
        return Ok(true);
    }
    if entry.rollback {
        entry.rollback = false;
        let Some(previous) = entry.previous.take() else {
            return Err("Aucune version précédente conservée".into());
        };
        swap_back(dir, &previous.files).inspect_err(|_| entry.previous = Some(previous.clone()))?;
        entry.current = Some(previous);
        return Ok(true);
    }
    Ok(false)
}

/// Premier usage réussi du modèle actuel : l'ancienne version devient inutile et est supprimée.
pub fn confirm_use_in(store: &mut Store, id: &str, dir: &Path) -> bool {
    let Some(previous) = store.models.get_mut(id).and_then(|e| e.previous.take()) else {
        return false;
    };
    for f in &previous.files {
        let _ = std::fs::remove_file(sibling(dir, &f.name, ".previous"));
    }
    true
}

// --- Registre sur disque et valeurs épinglées ---

/// Valeurs livrées avec cette version de Navi.
pub fn pinned(id: &str) -> Option<Installed> {
    match id {
        "kokoro" => Some(Installed {
            version: "1.0-int8".into(),
            files: crate::tts::install::pinned_files(),
        }),
        "whisper" => Some(Installed {
            version: "large-v3-turbo".into(),
            files: vec![crate::stt::model::pinned_file()],
        }),
        _ => None,
    }
}

/// Préfixe d'URL autorisé pour les téléchargements de `id` (le manifeste ne peut pas désigner un autre hôte).
pub fn allowed_prefix(id: &str) -> Option<&'static str> {
    match id {
        "kokoro" => Some("https://github.com/thewh1teagle/kokoro-onnx/releases/download/"),
        "whisper" => Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/"),
        _ => None,
    }
}

pub fn dir_of(id: &str) -> Option<PathBuf> {
    match id {
        "kokoro" => {
            let settings = crate::config::AppConfig::load().unwrap_or_default();
            Some(crate::utils::resolve_models_dir_with(
                settings.models_dir.as_deref(),
            ))
        }
        "whisper" => crate::stt::model::model_path()
            .parent()
            .map(Path::to_path_buf),
        _ => None,
    }
}

static LOCK: Mutex<()> = Mutex::new(());

fn store_path() -> Option<PathBuf> {
    crate::utils::app_data_dir().map(|d| d.join("installed-models.json"))
}

pub fn read() -> Store {
    store_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Lit le registre, applique `f`, puis l'écrit de façon atomique (fichier temporaire puis renommage).
pub fn update<R>(f: impl FnOnce(&mut Store) -> R) -> Result<R, String> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = store_path().ok_or("Dossier de données introuvable")?;
    let mut store = read();
    let result = f(&mut store);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(
        &tmp,
        serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?,
    )
    .and_then(|()| std::fs::rename(&tmp, &path))
    .map_err(|e| format!("{} : {e}", path.display()))?;
    Ok(result)
}

/// Fichiers attendus pour `id` (registre puis valeurs épinglées).
pub fn files(id: &str) -> Vec<FileSpec> {
    let pinned = pinned(id).expect("modèle géré");
    read().files(id, &pinned)
}

/// Empreinte du fichier principal : l'identité de la version installée, comparée au manifeste.
pub fn identity(id: &str) -> String {
    files(id)
        .first()
        .map(|f| f.sha256.clone())
        .unwrap_or_default()
}

/// Au démarrage, avant tout chargement : applique les mises à jour et retours arrière en attente.
pub fn apply_pending() {
    for id in IDS {
        let (Some(dir), Some(pinned)) = (dir_of(id), pinned(id)) else {
            continue;
        };
        match update(|s| apply_pending_in(s, id, &dir, &pinned)) {
            Ok(Ok(true)) => eprintln!("[navi] modèle {id} : changement de version appliqué"),
            Ok(Ok(false)) => {}
            Ok(Err(e)) | Err(e) => eprintln!("[navi] modèle {id} : {e}"),
        }
    }
}

/// À appeler après un usage réussi du modèle (chargement Kokoro, transcription Whisper).
pub fn confirm_use(id: &str) {
    if !read().has_previous(id) {
        return;
    }
    if let Some(dir) = dir_of(id)
        && let Err(e) = update(|s| confirm_use_in(s, id, &dir))
    {
        eprintln!("[navi] modèle {id} : nettoyage de l'ancienne version impossible : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &str, size: u64) -> FileSpec {
        FileSpec {
            name: name.into(),
            url: "https://exemple/x".into(),
            size,
            sha256: "aa".into(),
        }
    }

    fn inst(version: &str, size: u64) -> Installed {
        Installed {
            version: version.into(),
            files: vec![spec("m.bin", size)],
        }
    }

    fn setup(dir: &Path, old: &[u8], new: &[u8]) -> (Store, Installed) {
        std::fs::write(dir.join("m.bin"), old).unwrap();
        std::fs::write(dir.join("m.bin.new"), new).unwrap();
        let mut store = Store::default();
        store.stage_update("x", inst("2", new.len() as u64));
        (store, inst("1", old.len() as u64))
    }

    #[test]
    fn valeurs_epinglees_tant_que_rien_n_est_installe() {
        let store = Store::default();
        let pinned = inst("1", 3);
        assert_eq!(store.files("x", &pinned), pinned.files);
        assert_eq!(store.version("x", &pinned), "1");
        assert!(!store.has_previous("x") && store.pending("x").is_none());
    }

    #[test]
    fn mise_a_jour_appliquee_avec_ancienne_version_conservee() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, pinned) = setup(dir.path(), b"vieux", b"nouveau!");
        assert_eq!(store.pending("x"), Some("update"));
        assert!(apply_pending_in(&mut store, "x", dir.path(), &pinned).unwrap());
        assert_eq!(
            std::fs::read(dir.path().join("m.bin")).unwrap(),
            b"nouveau!"
        );
        assert_eq!(
            std::fs::read(dir.path().join("m.bin.previous")).unwrap(),
            b"vieux"
        );
        assert_eq!(store.version("x", &pinned), "2");
        assert!(store.has_previous("x") && store.pending("x").is_none());
    }

    #[test]
    fn mise_a_jour_incomplete_abandonnee_sans_toucher_au_modele() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, pinned) = setup(dir.path(), b"vieux", b"nouveau!");
        std::fs::write(dir.path().join("m.bin.new"), b"tronque").unwrap(); // taille inattendue
        assert!(apply_pending_in(&mut store, "x", dir.path(), &pinned).is_err());
        assert_eq!(std::fs::read(dir.path().join("m.bin")).unwrap(), b"vieux");
        assert!(!dir.path().join("m.bin.new").exists());
        assert_eq!(store.version("x", &pinned), "1");
        assert!(store.pending("x").is_none());
    }

    #[test]
    fn retour_arriere_restaure_la_version_precedente() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, pinned) = setup(dir.path(), b"vieux", b"nouveau!");
        apply_pending_in(&mut store, "x", dir.path(), &pinned).unwrap();
        store.stage_rollback("x").unwrap();
        assert_eq!(store.pending("x"), Some("rollback"));
        assert!(apply_pending_in(&mut store, "x", dir.path(), &pinned).unwrap());
        assert_eq!(std::fs::read(dir.path().join("m.bin")).unwrap(), b"vieux");
        assert!(!dir.path().join("m.bin.previous").exists());
        assert_eq!(store.version("x", &pinned), "1");
        assert!(!store.has_previous("x"));
    }

    #[test]
    fn retour_arriere_refuse_sans_version_precedente() {
        assert!(Store::default().stage_rollback("x").is_err());
    }

    #[test]
    fn ancienne_version_supprimee_apres_usage_reussi() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, pinned) = setup(dir.path(), b"vieux", b"nouveau!");
        apply_pending_in(&mut store, "x", dir.path(), &pinned).unwrap();
        assert!(confirm_use_in(&mut store, "x", dir.path()));
        assert!(!dir.path().join("m.bin.previous").exists());
        assert!(!store.has_previous("x"));
        assert!(
            !confirm_use_in(&mut store, "x", dir.path()),
            "rien à nettoyer la seconde fois"
        );
    }
}
