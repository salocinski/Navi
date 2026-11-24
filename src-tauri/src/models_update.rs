//! Contrôle de disponibilité de mise à jour des modèles (carte N-066).
//!
//! Notification seule : le manifeste public du dépôt Navi est lu en HTTPS, comparé aux empreintes des modèles
//! installés, et rien n'est installé (le remplacement par l'utilisateur est la carte N-098). La requête est
//! anonyme : aucun identifiant, texte ni son n'est envoyé.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/salocinski/Navi/main/manifests/models.json";
/// Version de format de modèle que ce moteur sait charger : au-delà, il faut mettre Navi à jour.
const SUPPORTED_FORMAT: u32 = 1;
const WEEK_SECS: u64 = 7 * 24 * 3600;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

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
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ModelUpdate {
    pub id: String,
    pub version: String,
    /// Une version différente existe et ce moteur sait la charger.
    pub available: bool,
    /// Une version différente existe mais exige une version plus récente de Navi.
    pub needs_app_update: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct CheckResult {
    pub checked_at: u64,
    pub models: Vec<ModelUpdate>,
}

static LAST: Mutex<Option<CheckResult>> = Mutex::new(None);

/// Dernier résultat de la session (contrôle hebdomadaire ou bouton), `None` avant tout contrôle.
pub fn last() -> Option<CheckResult> {
    LAST.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Modèles installés : (identifiant du manifeste, empreinte SHA-256 épinglée dans l'application).
fn installed() -> [(&'static str, &'static str); 2] {
    [
        ("kokoro", crate::tts::install::model_sha256()),
        ("whisper", crate::stt::model::MODEL_SHA256),
    ]
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

fn compare(manifest_json: &str, installed: &[(&str, &str)]) -> Result<Vec<ModelUpdate>, String> {
    let manifest: Manifest =
        serde_json::from_str(manifest_json).map_err(|e| format!("Manifeste illisible : {e}"))?;
    Ok(manifest
        .models
        .into_iter()
        .filter_map(|entry| {
            let (_, current) = installed.iter().find(|(id, _)| *id == entry.id)?;
            let differs = !entry.sha256.eq_ignore_ascii_case(current);
            let loadable = entry.format <= SUPPORTED_FORMAT;
            Some(ModelUpdate {
                id: entry.id,
                version: entry.version,
                available: differs && loadable,
                needs_app_update: differs && !loadable,
            })
        })
        .collect())
}

fn fetch_manifest() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let response = agent
        .get(MANIFEST_URL)
        .call()
        .map_err(|e| format!("Vérification impossible : {e}"))?;
    response
        .into_body()
        .with_config()
        .limit(MAX_MANIFEST_BYTES)
        .read_to_string()
        .map_err(|e| format!("Manifeste illisible : {e}"))
}

/// Interroge le manifeste, mémorise le résultat et la date du contrôle dans les réglages.
pub fn run_check() -> Result<CheckResult, String> {
    let models = compare(&fetch_manifest()?, &installed())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALLED: [(&str, &str); 2] = [("kokoro", "aaa"), ("whisper", "bbb")];

    fn manifest(entries: &str) -> String {
        format!(r#"{{"models":[{entries}]}}"#)
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

    /// Le manifeste publié dans le dépôt doit refléter les modèles épinglés dans cette version.
    #[test]
    fn manifeste_du_depot_coherent_avec_les_modeles_epingles() {
        let json = include_str!("../../manifests/models.json");
        let r = compare(json, &installed()).unwrap();
        assert_eq!(r.len(), 2);
        assert!(
            r.iter().all(|m| !m.available && !m.needs_app_update),
            "{r:?}"
        );
    }
}
