//! Contrôle et téléchargement des mises à jour des modèles (cartes N-066 et N-098).
//!
//! Le manifeste public du dépôt Navi est lu en HTTPS ; sa **signature ed25519** (clé publique fixée ci-dessous, clé
//! privée hors dépôt, voir `tools/sign-manifest.mjs`) est vérifiée avant toute utilisation. Le contrôle de disponibilité
//! n'installe rien ; le téléchargement d'une mise à jour est une action explicite de l'utilisateur, limitée aux hôtes
//! prévus pour chaque modèle, avec taille plafonnée et SHA-256 vérifié, puis appliquée au démarrage suivant
//! (`models_store`). Les requêtes sont anonymes : aucun identifiant, texte ni son n'est envoyé.

use crate::models_store::{self, FileSpec, Installed};
use ring::signature::{ED25519, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/salocinski/Navi/main/manifests/models.json";
/// Clé publique ed25519 qui signe `manifests/models.json`.
/// shortcut: une seule clé fixée dans le code ; une clé perdue ou compromise impose une nouvelle version de Navi.
const PUBLIC_KEY: [u8; 32] = [
    0xc0, 0x5d, 0x39, 0x41, 0xda, 0x7a, 0x13, 0x82, 0x80, 0x67, 0x25, 0x47, 0x47, 0x44, 0xd9, 0x89,
    0x4d, 0xc6, 0x23, 0x62, 0xd5, 0xab, 0x5b, 0xb4, 0xd5, 0xea, 0xc7, 0x08, 0xc7, 0xd5, 0x0c, 0x3d,
];
/// Version de format de modèle que ce moteur sait charger : au-delà, il faut mettre Navi à jour.
const SUPPORTED_FORMAT: u32 = 1;
const WEEK_SECS: u64 = 7 * 24 * 3600;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_SIGNATURE_BYTES: u64 = 1024;
/// Plafond de taille d'un fichier de modèle annoncé par le manifeste.
const MAX_MODEL_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Deserialize)]
struct Manifest {
    models: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    version: String,
    format: u32,
    sha256: String,
    #[serde(default)]
    files: Vec<ManifestFile>,
}

#[derive(Deserialize)]
struct ManifestFile {
    name: String,
    url: String,
    size: u64,
    sha256: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ModelUpdate {
    pub id: String,
    pub version: String,
    /// Une version différente existe et ce moteur sait la charger.
    pub available: bool,
    /// Une version différente existe mais exige une version plus récente de Navi.
    pub needs_app_update: bool,
    /// Octets à télécharger pour cette version (somme des fichiers du manifeste).
    pub download_bytes: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct CheckResult {
    pub checked_at: u64,
    pub models: Vec<ModelUpdate>,
}

static LAST: Mutex<Option<CheckResult>> = Mutex::new(None);
static DOWNLOADING: AtomicBool = AtomicBool::new(false);
static DONE: AtomicU64 = AtomicU64::new(0);
static TOTAL: AtomicU64 = AtomicU64::new(0);

/// Dernier résultat de la session (contrôle hebdomadaire ou bouton), `None` avant tout contrôle.
pub fn last() -> Option<CheckResult> {
    LAST.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// (téléchargement en cours, octets reçus, octets attendus)
pub fn progress() -> (bool, u64, u64) {
    (
        DOWNLOADING.load(Ordering::SeqCst),
        DONE.load(Ordering::Relaxed),
        TOTAL.load(Ordering::Relaxed),
    )
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Vrai si le contrôle hebdomadaire est activé et que le dernier date de plus de sept jours (ou n'a jamais eu lieu).
pub fn is_due(enabled: bool, last_check: Option<u64>, now: u64) -> bool {
    enabled && last_check.is_none_or(|t| now.saturating_sub(t) >= WEEK_SECS)
}

// --- Signature ---

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if !text.is_ascii() || !text.len().is_multiple_of(2) {
        return Err("Signature illisible".into());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| "Signature illisible".into()))
        .collect()
}

fn verify_with_key(key: &[u8], manifest: &[u8], signature_hex: &str) -> Result<(), String> {
    let signature = decode_hex(signature_hex)?;
    UnparsedPublicKey::new(&ED25519, key)
        .verify(manifest, &signature)
        .map_err(|_| "Signature du manifeste invalide : opération refusée".to_string())
}

fn http_get(url: &str, limit: u64) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("Vérification impossible : {e}"))?;
    response
        .into_body()
        .with_config()
        .limit(limit)
        .read_to_string()
        .map_err(|e| format!("Réponse illisible : {e}"))
}

/// Manifeste dont la signature est valide ; sinon erreur (aucun contenu non signé n'est exploité).
fn fetch_signed() -> Result<String, String> {
    let manifest = http_get(MANIFEST_URL, MAX_MANIFEST_BYTES)?;
    let signature = http_get(&format!("{MANIFEST_URL}.sig"), MAX_SIGNATURE_BYTES)?;
    verify_with_key(&PUBLIC_KEY, manifest.as_bytes(), &signature)?;
    Ok(manifest)
}

// --- Contrôle de disponibilité (N-066) ---

fn parse(manifest_json: &str) -> Result<Manifest, String> {
    serde_json::from_str(manifest_json).map_err(|e| format!("Manifeste illisible : {e}"))
}

fn compare<S: AsRef<str>>(
    manifest_json: &str,
    installed: &[(&str, S)],
) -> Result<Vec<ModelUpdate>, String> {
    Ok(parse(manifest_json)?
        .models
        .into_iter()
        .filter_map(|entry| {
            let (_, current) = installed.iter().find(|(id, _)| *id == entry.id)?;
            let differs = !entry.sha256.eq_ignore_ascii_case(current.as_ref());
            let loadable = entry.format <= SUPPORTED_FORMAT;
            let download_bytes = entry
                .files
                .iter()
                .fold(0u64, |total, f| total.saturating_add(f.size));
            Some(ModelUpdate {
                id: entry.id,
                version: entry.version,
                available: differs && loadable,
                needs_app_update: differs && !loadable,
                download_bytes,
            })
        })
        .collect())
}

/// Interroge le manifeste signé, mémorise le résultat et la date du contrôle dans les réglages.
pub fn run_check() -> Result<CheckResult, String> {
    let installed: Vec<(&str, String)> = models_store::IDS
        .iter()
        .map(|id| (*id, models_store::identity(id)))
        .collect();
    let models = compare(&fetch_signed()?, &installed)?;
    let result = CheckResult {
        checked_at: now(),
        models,
    };
    *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some(result.clone());
    let mut config = crate::config::AppConfig::load().unwrap_or_default();
    config.last_update_check = Some(result.checked_at);
    if let Err(e) = config.save() {
        eprintln!("[navi] date du contrôle de mise à jour non enregistrée : {e}");
    }
    Ok(result)
}

// --- Téléchargement d'une mise à jour (N-098) ---

fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Valide la mise à jour annoncée pour `id` et retourne les fichiers à télécharger, dans l'ordre des fichiers livrés.
/// Les noms viennent des fichiers livrés (jamais du manifeste) : aucun chemin arbitraire n'est écrit.
fn plan(
    id: &str,
    pinned: &[FileSpec],
    installed_sha256: &str,
    entry: &Entry,
) -> Result<Vec<FileSpec>, String> {
    if entry.format > SUPPORTED_FORMAT {
        return Err("Ce modèle nécessite une version plus récente de Navi".into());
    }
    if entry.sha256.eq_ignore_ascii_case(installed_sha256) {
        return Err("Ce modèle est déjà à jour".into());
    }
    let prefix = models_store::allowed_prefix(id).ok_or("Modèle inconnu")?;
    if entry.files.len() != pinned.len() {
        return Err("Manifeste incohérent : liste de fichiers inattendue".into());
    }
    let mut planned = Vec::new();
    for expected in pinned {
        let file = entry
            .files
            .iter()
            .find(|f| f.name == expected.name)
            .ok_or_else(|| format!("Fichier {} absent du manifeste", expected.name))?;
        if !file.url.starts_with(prefix) {
            return Err(format!("Source non autorisée pour {}", file.name));
        }
        if file.size == 0 || file.size > MAX_MODEL_BYTES {
            return Err(format!("Taille inattendue pour {}", file.name));
        }
        if !is_sha256(&file.sha256) {
            return Err(format!("Empreinte invalide pour {}", file.name));
        }
        planned.push(FileSpec {
            name: expected.name.clone(),
            url: file.url.clone(),
            size: file.size,
            sha256: file.sha256.to_ascii_lowercase(),
        });
    }
    if !planned[0].sha256.eq_ignore_ascii_case(&entry.sha256) {
        return Err("Manifeste incohérent : empreinte du fichier principal".into());
    }
    Ok(planned)
}

/// Télécharge et vérifie la mise à jour de `id` à côté du modèle actif (`.new`) ; elle sera appliquée au prochain
/// démarrage de Navi. Action explicite de l'utilisateur (bloquant, un seul téléchargement à la fois).
pub fn stage_update(id: &str) -> Result<(), String> {
    if models_store::pinned(id).is_none() {
        return Err("Modèle inconnu".into());
    }
    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return Err("Un téléchargement de modèle est déjà en cours".into());
    }
    let result = stage_files(id);
    DOWNLOADING.store(false, Ordering::SeqCst);
    result
}

fn stage_files(id: &str) -> Result<(), String> {
    let entry = parse(&fetch_signed()?)?
        .models
        .into_iter()
        .find(|e| e.id == id)
        .ok_or("Ce modèle n'est pas dans le manifeste")?;
    let pinned = models_store::pinned(id).ok_or("Modèle inconnu")?;
    let files = plan(id, &pinned.files, &models_store::identity(id), &entry)?;
    let dir = models_store::dir_of(id).ok_or("Dossier des modèles introuvable")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{} : {e}", dir.display()))?;

    TOTAL.store(files.iter().map(|f| f.size).sum(), Ordering::Relaxed);
    DONE.store(0, Ordering::Relaxed);
    let mut base = 0;
    for file in &files {
        let target = dir.join(format!("{}.new", file.name));
        let downloaded = crate::stt::model::download_to(
            &file.url,
            &target,
            file.size,
            &file.sha256,
            |done, _| DONE.store(base + done, Ordering::Relaxed),
        );
        if let Err(e) = downloaded {
            for f in &files {
                let _ = std::fs::remove_file(dir.join(format!("{}.new", f.name)));
            }
            return Err(e);
        }
        base += file.size;
    }
    models_store::update(|s| {
        s.stage_update(
            id,
            Installed {
                version: entry.version.clone(),
                files,
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    const INSTALLED: [(&str, &str); 2] = [("kokoro", "aaa"), ("whisper", "bbb")];
    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn manifest(entries: &str) -> String {
        format!(r#"{{"models":[{entries}]}}"#)
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn keypair() -> Ed25519KeyPair {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()
    }

    #[test]
    fn modele_a_jour_n_est_pas_signale() {
        let json = manifest(r#"{"id":"kokoro","version":"1","format":1,"sha256":"AAA"}"#);
        let r = compare(&json, &INSTALLED).unwrap();
        assert_eq!(r.len(), 1);
        assert!(!r[0].available && !r[0].needs_app_update);
    }

    #[test]
    fn nouvelle_empreinte_signalee_disponible() {
        let json = manifest(r#"{"id":"whisper","version":"2","format":1,"sha256":"ccc"}"#);
        let r = compare(&json, &INSTALLED).unwrap();
        assert!(r[0].available && !r[0].needs_app_update);
        assert_eq!(r[0].version, "2");
    }

    #[test]
    fn format_trop_recent_demande_la_mise_a_jour_de_l_application() {
        let json = manifest(r#"{"id":"kokoro","version":"9","format":2,"sha256":"zzz"}"#);
        let r = compare(&json, &INSTALLED).unwrap();
        assert!(!r[0].available && r[0].needs_app_update);
    }

    #[test]
    fn modele_inconnu_ignore_et_manifeste_invalide_refuse() {
        let json = manifest(r#"{"id":"autre","version":"1","format":1,"sha256":"x"}"#);
        assert!(compare(&json, &INSTALLED).unwrap().is_empty());
        assert!(compare("pas du json", &INSTALLED).is_err());
    }

    #[test]
    fn echeance_hebdomadaire() {
        let t = 10 * WEEK_SECS;
        assert!(!is_due(false, None, t), "désactivé : jamais");
        assert!(is_due(true, None, t), "jamais contrôlé");
        assert!(!is_due(true, Some(t - WEEK_SECS + 1), t));
        assert!(is_due(true, Some(t - WEEK_SECS), t));
        assert!(
            !is_due(true, Some(t + 5), t),
            "horloge reculée : pas de rafale"
        );
    }

    #[test]
    fn signature_valide_acceptee_alteree_ou_etrangere_refusee() {
        let (pair, other) = (keypair(), keypair());
        let data = b"{\"models\":[]}";
        let sig = hex(pair.sign(data).as_ref());
        let key = pair.public_key().as_ref();
        assert!(verify_with_key(key, data, &sig).is_ok());
        assert!(
            verify_with_key(key, b"{\"models\":[1]}", &sig).is_err(),
            "manifeste altéré"
        );
        assert!(
            verify_with_key(other.public_key().as_ref(), data, &sig).is_err(),
            "autre clé"
        );
        assert!(verify_with_key(key, data, "pas-de-l-hexadecimal").is_err());
        assert!(verify_with_key(key, data, "").is_err());
    }

    /// Le manifeste du dépôt doit porter une signature valide de la clé fixée dans l'application, et décrire les
    /// modèles épinglés dans cette version : modifier `models.json` sans le re-signer fait échouer ce test.
    #[test]
    fn manifeste_du_depot_signe_et_coherent_avec_les_modeles_epingles() {
        let json = include_str!("../../manifests/models.json");
        let sig = include_str!("../../manifests/models.json.sig");
        verify_with_key(&PUBLIC_KEY, json.as_bytes(), sig).expect("signature du dépôt valide");
        let pinned: Vec<(&str, String)> = models_store::IDS
            .iter()
            .map(|id| {
                (
                    *id,
                    models_store::pinned(id).unwrap().files[0].sha256.clone(),
                )
            })
            .collect();
        let r = compare(json, &pinned).unwrap();
        assert_eq!(r.len(), 2);
        assert!(
            r.iter().all(|m| !m.available && !m.needs_app_update),
            "{r:?}"
        );
    }

    fn entry(json: &str) -> Entry {
        serde_json::from_str(json).unwrap()
    }

    fn pinned_kokoro() -> Vec<FileSpec> {
        crate::tts::install::pinned_files()
    }

    fn kokoro_entry(patch: impl Fn(&mut serde_json::Value)) -> Entry {
        let prefix = models_store::allowed_prefix("kokoro").unwrap();
        let mut v = serde_json::json!({
            "id": "kokoro", "version": "2", "format": 1, "sha256": SHA_A,
            "files": [
                {"name": "kokoro-v1.0.int8.onnx", "url": format!("{prefix}x/a.onnx"), "size": 10, "sha256": SHA_A},
                {"name": "voices-v1.0.bin", "url": format!("{prefix}x/v.bin"), "size": 5, "sha256": SHA_A}
            ]
        });
        patch(&mut v);
        entry(&v.to_string())
    }

    #[test]
    fn plan_accepte_une_mise_a_jour_conforme() {
        let files = plan("kokoro", &pinned_kokoro(), "autre", &kokoro_entry(|_| {})).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "kokoro-v1.0.int8.onnx");
        assert_eq!(files[0].size, 10);
    }

    #[test]
    fn plan_refuse_les_manifestes_dangereux_ou_incoherents() {
        let p = pinned_kokoro();
        let err = |patch: &dyn Fn(&mut serde_json::Value)| {
            plan("kokoro", &p, "autre", &kokoro_entry(patch)).unwrap_err()
        };
        assert!(err(&|v| v["format"] = 2.into()).contains("plus récente"));
        assert!(
            err(&|v| v["files"][0]["url"] = "https://exemple.com/m.onnx".into()).contains("Source")
        );
        assert!(err(&|v| v["files"][0]["size"] = 0.into()).contains("Taille"));
        assert!(err(&|v| v["files"][0]["size"] = u64::MAX.into()).contains("Taille"));
        assert!(err(&|v| v["files"][1]["sha256"] = "xyz".into()).contains("Empreinte"));
        assert!(err(&|v| v["files"][0]["name"] = "..\\evil.onnx".into()).contains("absent"));
        assert!(
            err(&|v| {
                v["files"].as_array_mut().unwrap().pop();
            })
            .contains("incohérent")
        );
        assert!(err(&|v| v["sha256"] = "b".repeat(64).into()).contains("principal"));
        let same = plan("kokoro", &p, SHA_A, &kokoro_entry(|_| {})).unwrap_err();
        assert!(same.contains("déjà à jour"));
    }
}
