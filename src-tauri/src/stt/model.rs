//! Modèle de dictée (carte N-094) : emplacement, téléchargement et intégrité SHA-256.
//!
//! Le fichier (1,5 Go) est trop lourd pour l'installateur : il est téléchargé au premier usage depuis
//! Hugging Face. Avec le modèle de lecture (N-066) et le contrôle de mise à jour, ce sont les seules
//! communications réseau de Navi ; aucun texte ni son n'est envoyé.

use super::whisper::MODEL_FILE;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin";
pub const MODEL_SIZE: u64 = 1_624_555_275;
pub const MODEL_SHA256: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

/// Valeurs livrées avec cette version de Navi.
pub fn pinned_file() -> crate::models_store::FileSpec {
    crate::models_store::FileSpec {
        name: MODEL_FILE.into(),
        url: MODEL_URL.into(),
        size: MODEL_SIZE,
        sha256: MODEL_SHA256.into(),
    }
}

/// Fichier attendu : celui d'une mise à jour installée (N-098), sinon celui livré.
fn current() -> crate::models_store::FileSpec {
    crate::models_store::files("whisper")
        .into_iter()
        .next()
        .unwrap_or_else(pinned_file)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ModelState {
    Missing,
    /// Présent mais taille ou empreinte inattendue.
    Corrupt,
    Valid,
}

/// `NAVI_STT_MODEL` : modèle fourni à la main (développement), ni vérifié ni téléchargé.
pub fn is_overridden() -> bool {
    crate::utils::dev_env("NAVI_STT_MODEL").is_some()
}

pub fn model_path() -> PathBuf {
    if let Some(p) = crate::utils::dev_env("NAVI_STT_MODEL") {
        return p.into();
    }
    crate::utils::app_data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("models")
        .join(MODEL_FILE)
}

/// Contrôle bon marché (existence + taille), sans hacher le fichier.
pub fn is_present(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|m| m.is_file() && (is_overridden() || m.len() == current().size))
}

fn hex_sha256(reader: &mut impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    std::io::copy(reader, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_with(path: &Path, size: u64, sha256: &str) -> Result<ModelState, String> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ModelState::Missing),
        Err(e) => return Err(format!("{} : {e}", path.display())),
    };
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len != size {
        return Ok(ModelState::Corrupt);
    }
    let sum = hex_sha256(&mut file).map_err(|e| format!("{} : {e}", path.display()))?;
    Ok(if sum == sha256 {
        ModelState::Valid
    } else {
        ModelState::Corrupt
    })
}

/// Taille puis SHA-256 du modèle (~4 s pour 1,5 Go).
pub fn verify(path: &Path) -> Result<ModelState, String> {
    let spec = current();
    verify_with(path, spec.size, &spec.sha256)
}

/// Copie `src` vers `dst` en hachant au fil de l'eau ; refuse tout flux de taille ou d'empreinte inattendue.
fn copy_verified(
    mut src: impl Read,
    dst: &mut impl Write,
    size: u64,
    sha256: &str,
    mut progress: impl FnMut(u64),
) -> Result<(), String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        let n = src
            .read(&mut buf)
            .map_err(|e| format!("Téléchargement interrompu : {e}"))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > size {
            return Err("Téléchargement refusé : fichier plus grand qu'attendu".into());
        }
        hasher.update(&buf[..n]);
        dst.write_all(&buf[..n])
            .map_err(|e| format!("Écriture impossible : {e}"))?;
        progress(done);
    }
    if done != size {
        return Err(format!("Téléchargement incomplet ({done}/{size} octets)"));
    }
    if format!("{:x}", hasher.finalize()) != sha256 {
        return Err("Téléchargement refusé : empreinte SHA-256 inattendue".into());
    }
    Ok(())
}

/// Télécharge le modèle vers `dest` (fichier `.part` puis renommage) ; `progress(reçu, total)`.
/// shortcut: pas de reprise ni de délai sur le corps (un téléchargement interrompu repart de zéro ;
/// une connexion qui se fige sans erreur reste en attente) ; ajouter `Range` si les coupures gênent.
pub fn download(dest: &Path, progress: impl FnMut(u64, u64)) -> Result<(), String> {
    let spec = current();
    download_to(&spec.url, dest, spec.size, &spec.sha256, progress)
}

/// Téléchargement vérifié (taille et SHA-256 exigés), partagé avec les modèles de lecture (N-066).
pub(crate) fn download_to(
    url: &str,
    dest: &Path,
    size: u64,
    sha256: &str,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    let dir = dest.parent().ok_or("Dossier de destination invalide")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let mut part = dest.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let result = (|| {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(20)))
            .build()
            .into();
        let response = agent
            .get(url)
            .call()
            .map_err(|e| format!("Téléchargement impossible : {e}"))?;
        let mut file =
            std::fs::File::create(&part).map_err(|e| format!("{} : {e}", part.display()))?;
        copy_verified(
            response.into_body().into_reader(),
            &mut file,
            size,
            sha256,
            |done| progress(done, size),
        )?;
        file.sync_all().map_err(|e| e.to_string())
    })();
    match result {
        Ok(()) => std::fs::rename(&part, dest).map_err(|e| format!("{} : {e}", dest.display())),
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"contenu du modele";
    fn sha(data: &[u8]) -> String {
        hex_sha256(&mut &data[..]).unwrap()
    }

    #[test]
    fn copie_conforme_acceptee_avec_progression() {
        let mut out = Vec::new();
        let mut last = 0;
        copy_verified(DATA, &mut out, DATA.len() as u64, &sha(DATA), |d| last = d).unwrap();
        assert_eq!(out, DATA);
        assert_eq!(last, DATA.len() as u64);
    }

    #[test]
    fn copie_refusee_si_empreinte_taille_ou_depassement() {
        let size = DATA.len() as u64;
        let mut out = Vec::new();
        let bad_sha = copy_verified(DATA, &mut out, size, &sha(b"autre"), |_| {});
        assert!(bad_sha.unwrap_err().contains("SHA-256"));
        let short = copy_verified(&DATA[..4], &mut Vec::new(), size, &sha(DATA), |_| {});
        assert!(short.unwrap_err().contains("incomplet"));
        let long = copy_verified(DATA, &mut Vec::new(), size - 1, &sha(DATA), |_| {});
        assert!(long.unwrap_err().contains("plus grand"));
    }

    /// Téléchargement réel (1,5 Go) dans un dossier temporaire, puis vérification de l'empreinte.
    /// `cargo test telechargement_reel -- --ignored --nocapture` (réseau requis).
    #[test]
    #[ignore]
    fn telechargement_reel() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(MODEL_FILE);
        let mut last = 0;
        download(&dest, |done, total| {
            if done * 10 / total != last {
                last = done * 10 / total;
                eprintln!("{} %", last * 10);
            }
        })
        .unwrap();
        assert!(!dest.with_extension("bin.part").exists());
        assert_eq!(verify(&dest), Ok(ModelState::Valid));
    }

    #[test]
    fn verification_du_fichier_local() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("m.bin");
        let size = DATA.len() as u64;
        assert_eq!(verify_with(&p, size, &sha(DATA)), Ok(ModelState::Missing));
        std::fs::write(&p, DATA).unwrap();
        assert_eq!(verify_with(&p, size, &sha(DATA)), Ok(ModelState::Valid));
        assert_eq!(verify_with(&p, size, &sha(b"x")), Ok(ModelState::Corrupt));
        assert_eq!(
            verify_with(&p, size + 1, &sha(DATA)),
            Ok(ModelState::Corrupt)
        );
    }
}
