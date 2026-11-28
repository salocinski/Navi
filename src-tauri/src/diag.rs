//! Journalisation locale et centralisation des erreurs (carte N-034).
//!
//! Un seul fichier, rotation par taille, **aucun envoi distant** : le journal ne quitte
//! jamais l'ordinateur.
//!
//! Règle de confidentialité : on ne journalise jamais le texte lu, dicté ou reconnu — ni une
//! sélection, ni une transcription, ni un contenu de presse-papiers. Seulement des longueurs,
//! des langues, des identifiants de modèle et des messages d'erreur. Les lignes sont de toute
//! façon tronquées à [`MAX_LINE_CHARS`] : une journalisation accidentelle d'un texte reste
//! bornée.
//!
//! Emplacement : `<données de l'application>/logs/navi.log` (`%APPDATA%\Navi\logs` sous
//! Windows), avec `navi.log.1` pour le fichier précédent.
//!
//! Niveau : `Info` par défaut. En développement uniquement, `NAVI_LOG=debug` (ou `trace`,
//! `warn`, `error`, `off`) ajuste le niveau ; la variable est ignorée en release, comme les
//! autres variables de développement ([`crate::utils::dev_env`]).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Taille du journal au-delà de laquelle il est renommé en `.1` (2 Mio).
const MAX_BYTES: u64 = 2 * 1024 * 1024;
/// Longueur maximale d'une ligne : garde-fou contre la journalisation accidentelle d'un texte.
const MAX_LINE_CHARS: usize = 2_000;

const LOG_FILE: &str = "navi.log";
const PREVIOUS_FILE: &str = "navi.log.1";

/// Dossier des journaux (`None` si le dossier de données de l'application est introuvable).
pub fn log_dir() -> Option<PathBuf> {
    crate::utils::app_data_dir().map(|d| d.join("logs"))
}

/// Installe le journal de fichier et le crochet de panique.
///
/// À appeler une fois, au tout début de `run()`, avant Tauri : tout ce qui suit est journalisé.
pub fn init() {
    log::set_max_level(level_from_env());
    let _ = log::set_boxed_logger(Box::new(NaviLogger::new(log_dir(), MAX_BYTES)));
    install_panic_hook();
}

/// Niveau demandé par `NAVI_LOG` en développement, `Info` sinon.
fn level_from_env() -> log::LevelFilter {
    match crate::utils::dev_env("NAVI_LOG").as_deref().map(str::trim) {
        Some("off") => log::LevelFilter::Off,
        Some("error") => log::LevelFilter::Error,
        Some("warn") => log::LevelFilter::Warn,
        Some("debug") => log::LevelFilter::Debug,
        Some("trace") => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    }
}

/// Journalise les paniques avant l'arrêt : en release le binaire est compilé avec
/// `panic = "abort"`, et sans ce crochet la cause ne laisserait aucune trace sur disque.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "lieu inconnu".to_string());
        log::error!("panique à {location} : {}", payload_message(info.payload()));
        log::logger().flush();
        previous(info);
    }));
}

fn payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "message illisible".to_string()
    }
}

/// État du fichier courant : `None` tant que le fichier n'a pas pu être ouvert.
struct State {
    file: Option<File>,
    size: u64,
}

/// Journal : fichier avec rotation, doublé de `stderr` en développement.
///
/// `dir == None` (dossier de données introuvable ou non inscriptible) : le journal se limite
/// à `stderr`, la journalisation ne casse jamais l'application.
struct NaviLogger {
    dir: Option<PathBuf>,
    max_bytes: u64,
    state: Mutex<State>,
}

impl NaviLogger {
    fn new(dir: Option<PathBuf>, max_bytes: u64) -> Self {
        let (dir, file) = match dir {
            Some(dir) => match open_log(&dir) {
                Some(file) => (Some(dir), Some(file)),
                None => (None, None),
            },
            None => (None, None),
        };
        let size = file
            .as_ref()
            .and_then(|f| f.metadata().ok())
            .map(|m| m.len())
            .unwrap_or(0);
        Self {
            dir,
            max_bytes,
            state: Mutex::new(State { file, size }),
        }
    }

    /// Écrit une ligne, en tournant le fichier si la taille maximale est atteinte.
    fn write(&self, line: &str) {
        let Some(dir) = self.dir.as_deref() else {
            return;
        };
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        let bytes = line.len() as u64;
        if state.size + bytes > self.max_bytes {
            // Le fichier doit être fermé avant le renommage (verrou Windows).
            state.file = None;
            rotate(dir);
            state.file = open_log(dir);
            state.size = state
                .file
                .as_ref()
                .and_then(|f| f.metadata().ok())
                .map(|m| m.len())
                .unwrap_or(0);
        }
        if let Some(file) = state.file.as_mut()
            && file.write_all(line.as_bytes()).is_ok()
        {
            state.size += bytes;
        }
    }
}

impl log::Log for NaviLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let mut line = format!(
            "{} {:<5} {}: {}",
            timestamp(),
            record.level(),
            record.target(),
            record.args()
        );
        truncate(&mut line, MAX_LINE_CHARS);
        line.push('\n');
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
        self.write(&line);
    }

    fn flush(&self) {
        if let Ok(mut state) = self.state.lock()
            && let Some(file) = state.file.as_mut()
        {
            let _ = file.flush();
        }
    }
}

/// Ouvre (ou crée) le journal en ajout, après création du dossier.
fn open_log(dir: &Path) -> Option<File> {
    std::fs::create_dir_all(dir).ok()?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(LOG_FILE))
        .ok()
}

/// Renomme le journal courant en `.1`, en remplaçant le précédent : un seul fichier
/// d'historique est conservé.
fn rotate(dir: &Path) {
    let previous = dir.join(PREVIOUS_FILE);
    let _ = std::fs::remove_file(&previous);
    let _ = std::fs::rename(dir.join(LOG_FILE), &previous);
}

/// Borne la longueur d'une ligne (en caractères), en signalant la coupure.
fn truncate(line: &mut String, max_chars: usize) {
    if line.chars().count() <= max_chars {
        return;
    }
    let kept: String = line.chars().take(max_chars).collect();
    *line = format!("{kept}… (tronqué)");
}

/// Horodatage UTC ISO 8601 à la milliseconde : `2026-11-27T09:12:33.123Z`.
fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format_utc(now.as_secs(), now.subsec_millis())
}

fn format_utc(secs: u64, millis: u32) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        rest / 3_600,
        (rest % 3_600) / 60,
        rest % 60
    )
}

/// Jours depuis 1970-01-01 → (année, mois, jour).
///
/// Algorithme « civil_from_days » de Howard Hinnant (chrono-Compatible Low-Level Date
/// Algorithms) : pas de dépendance de calendrier pour un horodatage.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horodatage_epoch() {
        assert_eq!(format_utc(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(format_utc(0, 7), "1970-01-01T00:00:00.007Z");
    }

    #[test]
    fn horodatage_date_connue() {
        // 2023-11-14T22:13:20Z
        assert_eq!(format_utc(1_700_000_000, 123), "2023-11-14T22:13:20.123Z");
        // 2026-11-27T00:00:00Z
        assert_eq!(format_utc(1_795_737_600, 0), "2026-11-27T00:00:00.000Z");
    }

    #[test]
    fn ligne_tronquee_au_dela_de_la_limite() {
        let mut line = "a".repeat(10);
        truncate(&mut line, 4);
        assert_eq!(line, "aaaa… (tronqué)");
        let mut short = "abc".to_string();
        truncate(&mut short, 4);
        assert_eq!(short, "abc");
    }

    #[test]
    fn rotation_conserve_un_seul_fichier_precedent() {
        let dir = tempfile::tempdir().unwrap();
        let logger = NaviLogger::new(Some(dir.path().to_path_buf()), 64);
        for n in 0..5 {
            logger.write(&format!("ligne {n} — 0123456789012345678901234567890\n"));
        }
        let current = std::fs::read_to_string(dir.path().join(LOG_FILE)).unwrap();
        let previous = std::fs::read_to_string(dir.path().join(PREVIOUS_FILE)).unwrap();
        assert!(current.len() as u64 <= 64, "journal courant : {current:?}");
        assert_eq!(previous.lines().count(), 1, "un seul cycle conservé");
        assert!(previous.starts_with("ligne 3"), "précédent : {previous:?}");
        assert!(current.starts_with("ligne 4"), "courant : {current:?}");
    }

    #[test]
    fn dossier_introuvable_ne_casse_rien() {
        let logger = NaviLogger::new(None, 64);
        logger.write("ignoré\n");
        assert!(logger.state.lock().unwrap().file.is_none());
    }
}
