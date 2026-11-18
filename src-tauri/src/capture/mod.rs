//! Capture de texte sélectionné via presse-papiers
//!
//! Cas limites gérés (carte TW-014), chacun avec une erreur claire :
//! - aucune fenêtre au premier plan → `NoForegroundWindow`
//! - l'application ne traite pas WM_COPY dans le délai imparti
//!   (copie bloquée / app gelée) → `ApplicationHung`
//! - copie asynchrone : la capture attend jusqu'à 1,5 s que le
//!   presse-papiers change (numéro de séquence) au lieu d'un délai fixe
//!   → `NoSelection` si rien ne change
//! - presse-papiers verrouillé par une autre application → `ClipboardBusy`
//!   (5 tentatives × 100 ms)
//! - aucun format texte exploitable (ou texte vide) → `NoTextContent`
//! - interruption : fenêtre détruite pendant la capture / timeout → erreurs ci-dessus

use std::fmt;
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM, HGLOBAL, GetLastError};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SendMessageTimeoutW, SMTO_ABORTIFHUNG,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, GetClipboardSequenceNumber,
    IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

const WM_COPY: u32 = 0x0301;
const CF_UNICODETEXT: u32 = 13;
const CF_TEXT: u32 = 1;
const CF_HTML: u32 = 0xC00E;
const CF_RTF: u32 = 0xC00F;

/// Délai maximal de réponse à WM_COPY (app gelée / copie bloquée).
const COPY_MESSAGE_TIMEOUT_MS: u32 = 1200;
/// Attente maximale d'un changement du presse-papiers (copie asynchrone).
const CLIPBOARD_CHANGE_TIMEOUT: Duration = Duration::from_millis(1500);
/// Intervalle de sondage du numéro de séquence du presse-papiers.
const CLIPBOARD_POLL_INTERVAL: Duration = Duration::from_millis(25);
/// Tentatives d'ouverture du presse-papiers (verrou externe).
const OPEN_CLIPBOARD_ATTEMPTS: usize = 5;
const OPEN_CLIPBOARD_RETRY_DELAY: Duration = Duration::from_millis(100);

/// Erreurs claires de capture (carte TW-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    /// Aucune fenêtre au premier plan.
    NoForegroundWindow,
    /// L'application ne traite pas WM_COPY dans le délai imparti (app gelée / copie bloquée).
    ApplicationHung { timeout_ms: u32 },
    /// Le presse-papiers n'a pas changé après WM_COPY (aucune sélection ou copie non effectuée).
    NoSelection,
    /// Le presse-papiers est verrouillé par une autre application.
    ClipboardBusy { attempts: usize },
    /// Aucun format texte exploitable dans le presse-papiers (ou texte vide).
    NoTextContent,
    /// Erreur bas niveau au moment de lire un format.
    ReadFailure(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::NoForegroundWindow => {
                write!(f, "Aucune fenêtre au premier plan — impossible de cibler la sélection")
            }
            CaptureError::ApplicationHung { timeout_ms } => write!(
                f,
                "L'application au premier plan ne répond pas à la demande de copie (délai de {timeout_ms} ms dépassé — application gelée ou copie bloquée)"
            ),
            CaptureError::NoSelection => write!(
                f,
                "Aucune sélection détectée — le presse-papiers n'a pas changé après la copie (aucune sélection, ou copie non effectuée par l'application)"
            ),
            CaptureError::ClipboardBusy { attempts } => write!(
                f,
                "Presse-papiers inaccessible — une autre application le verrouille (après {attempts} tentatives). Fermez l'application qui l'utilise puis réessayez"
            ),
            CaptureError::NoTextContent => write!(
                f,
                "Aucun texte capturé — le presse-papiers ne contient pas de format texte exploitable (Unicode, HTML, RTF ou ANSI) ou il est vide"
            ),
            CaptureError::ReadFailure(detail) => {
                write!(f, "Lecture du presse-papiers impossible : {detail}")
            }
        }
    }
}

impl std::error::Error for CaptureError {}

/// Résultat d'une capture
#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub text: String,
    pub format_used: String,
    pub success: bool,
    pub error: Option<String>,
}

impl CaptureResult {
    pub fn success(text: String, format: String) -> Self {
        Self { text, format_used: format, success: true, error: None }
    }

    pub fn failure(err: CaptureError) -> Self {
        Self { text: String::new(), format_used: String::new(), success: false, error: Some(err.to_string()) }
    }
}

/// Attend que le numéro de séquence du presse-papiers change (copie asynchrone).
/// Retourne vrai si le presse-papiers a changé dans le délai imparti.
///
/// `sequence_number` est injectable pour les tests.
fn wait_for_clipboard_change(
    seq_before: u32,
    timeout: Duration,
    mut sequence_number: impl FnMut() -> u32,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if sequence_number() != seq_before {
            return true;
        }
        thread::sleep(CLIPBOARD_POLL_INTERVAL);
    }
    false
}

/// Capture le texte sélectionné via Ctrl+C et lecture presse-papiers.
///
/// Ne bloque jamais indéfiniment : un délai maximum est appliqué à chaque étape.
pub fn capture_selection() -> CaptureResult {
    match capture_selection_inner() {
        Ok(result) => result,
        Err(err) => CaptureResult::failure(err),
    }
}

fn capture_selection_inner() -> Result<CaptureResult, CaptureError> {
    // 1. Fenêtre au premier plan
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err(CaptureError::NoForegroundWindow);
    }

    // 2. Mémoriser le numéro de séquence du presse-papiers avant la copie
    let seq_before = unsafe { GetClipboardSequenceNumber() };

    // 3. Envoyer WM_COPY avec timeout (une app gelée ne doit pas bloquer la capture)
    let mut result: usize = 0;
    let send_ok = unsafe {
        SendMessageTimeoutW(
            HWND(hwnd.0),
            WM_COPY,
            WPARAM(0 as usize),
            LPARAM(0 as isize),
            SMTO_ABORTIFHUNG,
            COPY_MESSAGE_TIMEOUT_MS,
            Some(&mut result),
        )
    };
    if send_ok.0 == 0 {
        let last_error = unsafe { GetLastError() };
        // SendMessageTimeoutW retourne 0 : soit l'app est gelée (SMTO_ABORTIFHUNG),
        // soit une erreur système (fenêtre détruite, etc.).
        let _ = last_error;
        return Err(CaptureError::ApplicationHung { timeout_ms: COPY_MESSAGE_TIMEOUT_MS });
    }

    // 4. Copie asynchrone : attendre que le presse-papiers change
    if !wait_for_clipboard_change(seq_before, CLIPBOARD_CHANGE_TIMEOUT, || unsafe {
        GetClipboardSequenceNumber()
    }) {
        return Err(CaptureError::NoSelection);
    }

    // 5. Ouvrir le presse-papiers (retries : peut être verrouillé par une autre app)
    let mut opened = false;
    for attempt in 1..=OPEN_CLIPBOARD_ATTEMPTS {
        if unsafe { OpenClipboard(None) }.is_ok() {
            opened = true;
            break;
        }
        if attempt < OPEN_CLIPBOARD_ATTEMPTS {
            thread::sleep(OPEN_CLIPBOARD_RETRY_DELAY);
        }
    }
    if !opened {
        return Err(CaptureError::ClipboardBusy { attempts: OPEN_CLIPBOARD_ATTEMPTS });
    }

    // 6. Lire le contenu (priorité : Unicode > HTML > RTF > ANSI)
    let result = read_clipboard_content();

    // 7. Fermer le presse-papiers (toujours)
    let _ = unsafe { CloseClipboard() };

    result
}

fn read_clipboard_content() -> Result<CaptureResult, CaptureError> {
    let formats_priority = [
        (CF_UNICODETEXT, "Unicode"),
        (CF_HTML, "HTML"),
        (CF_RTF, "RTF"),
        (CF_TEXT, "ANSI"),
    ];

    for (fmt, name) in formats_priority {
        if unsafe { IsClipboardFormatAvailable(fmt) }.is_err() {
            continue;
        }
        match read_format_raw(fmt) {
            Ok(content) => {
                if !content.trim().is_empty() {
                    return Ok(CaptureResult::success(content, name.to_string()));
                }
            }
            Err(detail) => {
                // Un format illisible ne doit pas empêcher d'essayer les autres ;
                // l'échec final sera NoTextContent si tout est illisible.
                let _ = detail;
            }
        }
    }

    Err(CaptureError::NoTextContent)
}

fn read_format_raw(format: u32) -> Result<String, String> {
    let hglobal = unsafe { GetClipboardData(format).map_err(|e| e.to_string())? };
    let handle = HGLOBAL(hglobal.0);
    if handle.0.is_null() {
        return Err("GetClipboardData a retourné NULL".to_string());
    }

    let size = unsafe { GlobalSize(handle) };
    if size == 0 {
        return Err("GlobalSize a retourné 0".to_string());
    }

    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        return Err("GlobalLock a retourné NULL".to_string());
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, size as usize) };

    let result = if format == CF_UNICODETEXT {
        let utf16: Vec<u16> = slice
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        String::from_utf16(&utf16).map_err(|e| e.to_string())
    } else {
        Ok(String::from_utf8_lossy(slice).trim_end_matches('\0').to_string())
    };

    let _ = unsafe { GlobalUnlock(handle) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_result() {
        let success = CaptureResult::success("test".to_string(), "test".to_string());
        assert!(success.success);
        assert_eq!(success.text, "test");

        let error = CaptureResult::failure(CaptureError::NoSelection);
        assert!(!error.success);
        assert!(error.error.unwrap().contains("Aucune sélection"));
    }

    #[test]
    fn messages_derreur_clairs_pour_chaque_cas() {
        let cas = [
            CaptureError::NoForegroundWindow,
            CaptureError::ApplicationHung { timeout_ms: 1200 },
            CaptureError::NoSelection,
            CaptureError::ClipboardBusy { attempts: 5 },
            CaptureError::NoTextContent,
            CaptureError::ReadFailure("échec GlobalLock".to_string()),
        ];
        for err in cas {
            let msg = err.to_string();
            assert!(msg.len() > 10, "message trop court pour {err:?}");
            // Un message clair commence par la cause, jamais par "Error" générique.
            assert!(!msg.starts_with("Error"), "message non clair pour {err:?}");
        }
    }

    #[test]
    fn sequence_changee_est_detectee() {
        let mut seq = 100u32;
        let changed = wait_for_clipboard_change(100, Duration::from_millis(200), || {
            let current = seq;
            seq += 1; // simule une copie qui incrémente le numéro de séquence
            current + 1
        });
        assert!(changed);
    }

    #[test]
    fn sequence_inchangee_expire() {
        let changed = wait_for_clipboard_change(42, Duration::from_millis(80), || 42);
        assert!(!changed);
    }
}
