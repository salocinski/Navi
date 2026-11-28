//! Capture de texte sélectionné via presse-papiers
//!
//! Cas limites gérés (carte TW-014), chacun avec une erreur claire :
//! - aucune fenêtre au premier plan → `NoForegroundWindow`
//! - Navi au premier plan (bouton/panneau) et Windows refuse de redonner l'avant-plan
//!   à la fenêtre qui détient la sélection → `WindowSwitchFailed`
//! - la simulation Ctrl+C (SendInput) est refusée par le système → `InjectionFailed`
//! - copie asynchrone : la capture attend jusqu'à 1,5 s que le
//!   presse-papiers change (numéro de séquence) au lieu d'un délai fixe
//!   → `NoSelection` si rien ne change (aucune sélection, app qui ne répond pas)
//! - presse-papiers verrouillé par une autre application → `ClipboardBusy`
//! - aucun format texte exploitable (ou texte vide) → `NoTextContent`
//! - interruption : tous les chemins d'attente sont bornés
//!
//! Restauration du presse-papiers utilisateur (carte TW-043) :
//! un snapshot complet (tous les formats hors GDI) est pris **avant** Ctrl+C simul� (SendInput) ;
//! il est **toujours** restauré en sortie, y compris en cas d'erreur ou
//! d'interruption. Les formats non restaurables (handles GDI) sont ignorés
//! explicitement et comptés.

use std::fmt;
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{BOOL, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcessId, GetCurrentThreadId,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, SendInput, VK_C, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, keybd_event,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindow,
    SW_RESTORE, SetForegroundWindow, ShowWindow,
};

const CF_UNICODETEXT: u32 = 13;
const CF_TEXT: u32 = 1;
const CF_HTML: u32 = 0xC00E;
const CF_RTF: u32 = 0xC00F;

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
    /// La simulation Ctrl+C (SendInput) a été refusée par le système.
    InjectionFailed { sent: u32 },
    /// Le presse-papiers n'a pas changé après le Ctrl+C simulé (aucune sélection, ou copie non effectuée).
    NoSelection,
    /// Le presse-papiers est verrouillé par une autre application.
    ClipboardBusy { attempts: usize },
    /// Aucun format texte exploitable dans le presse-papiers (ou texte vide).
    NoTextContent,
    /// Erreur bas niveau au moment de lire un format.
    ReadFailure(String),
    /// Impossible de sauvegarder le presse-papiers avant la copie :
    /// sans snapshot, l'utilisateur perdrait son presse-papiers → capture refusée.
    SnapshotFailed(String),
    /// Navi avait le focus (clic sur bouton/panneau) et Windows a refusé de
    /// redonner l'avant-plan à la fenêtre qui détient la sélection.
    WindowSwitchFailed(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::NoForegroundWindow => {
                write!(
                    f,
                    "Aucune fenêtre au premier plan — impossible de cibler la sélection"
                )
            }
            CaptureError::InjectionFailed { sent } => write!(
                f,
                "Impossible de simuler Ctrl+C — Windows a bloqué l'injection clavier ({sent}/4 événements acceptés). Une protection (antivirus, sandbox, app élevée) l'a peut-être refusée, ou aucune fenêtre n'accepte l'entrée"
            ),
            CaptureError::NoSelection => write!(
                f,
                "Aucune sélection détectée — le presse-papiers n'a pas changé après Ctrl+C (aucune sélection, copie non effectuée, ou application qui ne répond pas)"
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
            CaptureError::SnapshotFailed(detail) => write!(
                f,
                "Sauvegarde du presse-papiers impossible avant la copie ({detail}). Capture annulée pour ne pas écraser le presse-papiers utilisateur"
            ),
            CaptureError::WindowSwitchFailed(detail) => write!(
                f,
                "Impossible d'activer la fenêtre qui contient la sélection ({detail}). Cliquez sur cette fenêtre puis relancez la lecture"
            ),
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
    /// `None` si le presse-papiers a été intégralement restauré (ou si la capture a échoué
    /// avant toute modification). `Some(message)` si la restauration a été partielle ou impossible.
    pub restore: Option<String>,
}

impl CaptureResult {
    pub fn success(text: String, format: String) -> Self {
        Self {
            text,
            format_used: format,
            success: true,
            error: None,
            restore: None,
        }
    }

    pub fn failure(err: CaptureError) -> Self {
        Self {
            text: String::new(),
            format_used: String::new(),
            success: false,
            error: Some(err.to_string()),
            restore: None,
        }
    }
}

/// Rapport de restauration (comptage par format, pour messages utilisateur et diagnostic).
#[derive(Debug, Clone, Default)]
pub struct RestoreReport {
    /// Nombre de formats sauvegardés dans le snapshot.
    pub snapshot_items: usize,
    /// Nombre de formats ignorés car non copiables (handles GDI / formats non-mémoire).
    pub skipped: usize,
    /// Nombre de formats réécrits avec succès.
    pub restored: usize,
    /// Formats qui n'ont pas pu être réécrits (code format).
    pub failed: Vec<u32>,
}

impl RestoreReport {
    pub fn summary(&self) -> Option<String> {
        if self.failed.is_empty() {
            None
        } else {
            Some(format!(
                "Restauration partielle du presse-papiers : {}/{} formats restaurés (échec : {})",
                self.restored,
                self.snapshot_items,
                self.failed
                    .iter()
                    .map(|f| format!("0x{f:04X}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    }
}

/// Un format du presse-papiers sauvegardé octet pour octet.
struct ClipboardItem {
    format: u32,
    bytes: Vec<u8>,
}

/// Snapshot complet du presse-papiers (hors formats GDI non copiables).
struct ClipboardSnapshot {
    items: Vec<ClipboardItem>,
    skipped: Vec<u32>,
}

/// Les formats qui sont des handles GDI (pas de mémoire globale copiable) :
/// CF_BITMAP, CF_METAFILEPICT, CF_PALETTE, CF_LOCALE (atom), CF_OWNERDISPLAY,
/// CF_DSP* (0x80–0x8F), et la plage GDIOBJFIRST–GDIOBJLAST (0x0300–0x03FF).
fn is_global_memory_format(format: u32) -> bool {
    if (0x0300..=0x03FF).contains(&format) {
        return false;
    }
    !matches!(
        format,
        0x0002 // CF_BITMAP
        | 0x0003 // CF_METAFILEPICT
        | 0x0009 // CF_PALETTE
        | 0x0010 // CF_LOCALE
        | 0x0080..=0x008F // CF_OWNERDISPLAY, CF_DSPTEXT, CF_DSPBITMAP, …
    )
}

/// Ouvre le presse-papiers avec retries (verrou externe).
fn open_clipboard() -> bool {
    for attempt in 1..=OPEN_CLIPBOARD_ATTEMPTS {
        if unsafe { OpenClipboard(None) }.is_ok() {
            return true;
        }
        if attempt < OPEN_CLIPBOARD_ATTEMPTS {
            thread::sleep(OPEN_CLIPBOARD_RETRY_DELAY);
        }
    }
    false
}

/// Lit les octets bruts d'un format (mémoire globale).
fn read_format_bytes(format: u32) -> Result<Vec<u8>, String> {
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
    let bytes = slice.to_vec();

    let _ = unsafe { GlobalUnlock(handle) };
    Ok(bytes)
}

/// Sauvegarde le contenu complet du presse-papiers (avant la copie).
///
/// Si le presse-papiers est inaccessible, la capture est refusée : mieux vaut
/// ne pas écraser le presse-papiers utilisateur que le détruire.
fn snapshot_clipboard() -> Result<ClipboardSnapshot, CaptureError> {
    if !open_clipboard() {
        return Err(CaptureError::SnapshotFailed(format!(
            "ouverture du presse-papiers impossible après {OPEN_CLIPBOARD_ATTEMPTS} tentatives"
        )));
    }

    let mut items = Vec::new();
    let mut skipped = Vec::new();

    let mut format = 0u32;
    loop {
        let next = unsafe { EnumClipboardFormats(format) };
        if next == 0 {
            break;
        }
        format = next;

        if !is_global_memory_format(format) {
            skipped.push(format);
            continue;
        }

        match read_format_bytes(format) {
            Ok(bytes) => items.push(ClipboardItem { format, bytes }),
            Err(_) => skipped.push(format),
        }
    }

    let _ = unsafe { CloseClipboard() };

    Ok(ClipboardSnapshot { items, skipped })
}

/// Restaure le presse-papiers à partir d'un snapshot.
/// Échec partiel toléré : les autres formats sont réécrits, l'échec est rapporté.
fn restore_clipboard(snapshot: &ClipboardSnapshot) -> RestoreReport {
    let mut report = RestoreReport {
        snapshot_items: snapshot.items.len(),
        skipped: snapshot.skipped.len(),
        restored: 0,
        failed: Vec::new(),
    };

    if snapshot.items.is_empty() {
        return report;
    }

    if !open_clipboard() {
        report.failed = snapshot.items.iter().map(|i| i.format).collect();
        return report;
    }

    let emptied = unsafe { EmptyClipboard() };
    if emptied.is_err() {
        report.failed = snapshot.items.iter().map(|i| i.format).collect();
        let _ = unsafe { CloseClipboard() };
        return report;
    }

    for item in &snapshot.items {
        let result = unsafe { GlobalAlloc(GMEM_MOVEABLE, item.bytes.len()) };
        let hglobal = match result {
            Ok(h) => h,
            Err(_) => {
                report.failed.push(item.format);
                continue;
            }
        };

        let ptr = unsafe { GlobalLock(hglobal) };
        if ptr.is_null() {
            report.failed.push(item.format);
            // Note : GlobalFree n'est pas exposée par windows crate 0.58 ;
            // en cas d'échec, ce handle n'est pas libéré (fuite bornée, chemin d'erreur rare).
            continue;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(item.bytes.as_ptr(), ptr as *mut u8, item.bytes.len());
        }
        let _ = unsafe { GlobalUnlock(hglobal) };

        // En cas de succès, le système devient propriétaire du handle.
        match unsafe {
            windows::Win32::System::DataExchange::SetClipboardData(item.format, HANDLE(hglobal.0))
        } {
            Ok(_) => report.restored += 1,
            Err(_) => report.failed.push(item.format),
        }
    }

    let _ = unsafe { CloseClipboard() };
    report
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
/// Le presse-papiers de l'utilisateur est sauvegardé **avant** la copie et
/// **toujours** restauré en sortie (succès, erreur, interruption).
/// Ne bloque jamais indéfiniment : un délai maximum est appliqué à chaque étape.
pub fn capture_selection() -> CaptureResult {
    // 1. Snapshot complet du presse-papiers AVANT la copie.
    let snapshot = match snapshot_clipboard() {
        Ok(s) => s,
        Err(err) => return CaptureResult::failure(err),
    };

    // 2. Capture (peut échouer : injection refusée, aucune sélection, lecture…).
    let result = capture_steps();

    // 3. Restauration systématique — même en cas d'erreur.
    let report = restore_clipboard(&snapshot);
    let restore_message = report.summary();

    match result {
        Ok(mut ok) => {
            ok.restore = restore_message;
            ok
        }
        Err(err) => {
            let mut failed = CaptureResult::failure(err);
            failed.restore = restore_message;
            failed
        }
    }
}

/// Simule Ctrl+C dans la fenêtre au premier plan (SendInput, 4 événements).
///
/// Contrairement à un `WM_COPY` envoyé à la fenêtre, ceci passe par la file de
/// messages clavier : c'est ce que les applications traitent réellement (Ctrl+C
/// est un accélérateur de menu, pas seulement un message de fenêtre).
fn send_copy_keystroke() -> Result<(), CaptureError> {
    let down = KEYBD_EVENT_FLAGS(0);
    let up = KEYEVENTF_KEYUP;

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_C,
                    wScan: 0,
                    dwFlags: down,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_C,
                    wScan: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: up,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(CaptureError::InjectionFailed { sent });
    }
    Ok(())
}

/// Attend que tous les modificateurs physiques soient relâchés avant d'injecter Ctrl+C.
///
/// Quand le raccourci Navi (ex. Win+Alt+S) est pressé, les touches Win/Alt sont encore
/// enfoncées au moment où la capture s'exécute : injecter Ctrl+C à ce moment-là revient
/// à envoyer « Win+Alt+Ctrl+C » à la cible, qui n'exécute pas la copie — le presse-papiers
/// ne change pas et la capture échoue. On attend donc le relâchement complet
/// (borné ; best effort au-delà, la copie échouera avec un message clair comme avant).
pub(crate) fn wait_for_modifiers_released(timeout: Duration) {
    const MODIFIERS: [windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY; 11] = [
        VK_CONTROL,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_MENU,
        VK_LMENU,
        VK_RMENU,
        VK_SHIFT,
        VK_LSHIFT,
        VK_RSHIFT,
        VK_LWIN,
        VK_RWIN,
    ];
    let start = Instant::now();
    while start.elapsed() < timeout {
        let any_down = MODIFIERS
            .iter()
            .any(|vk| (unsafe { GetAsyncKeyState(vk.0 as i32) } as u16) & 0x8000 != 0);
        if !any_down {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    log::debug!(
        "modificateurs encore enfoncés après {} ms — copie tentée quand même",
        timeout.as_millis()
    );
}

/// PID du processus propriétaire d'une fenêtre (0 si la fenêtre est null ou invalide).
fn window_pid(hwnd: HWND) -> u32 {
    if hwnd.0.is_null() {
        return 0;
    }
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

/// Force l'activation d'une fenêtre : deux astuces combinées, car Windows refuse
/// `SetForegroundWindow` aux processus sans droit d'avant-plan (cas Navi bouton/panneau
/// après réception d'un clic, ou raccourci global) :
/// - la touche Alt factice accorde le droit d'avant-plan au processus ;
/// - `AttachThreadInput` rattache le thread courant aux threads des deux fenêtres
///   (file d'entrée partagée → l'activation est honorée).
fn force_foreground(target: HWND) {
    unsafe {
        let current = GetCurrentThreadId();
        let foreground = GetForegroundWindow();
        let fg_thread = if foreground.0.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, None)
        };
        let target_thread = GetWindowThreadProcessId(target, None);

        keybd_event(VK_MENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);

        if fg_thread != 0 {
            let _ = AttachThreadInput(current, fg_thread, BOOL(1));
        }
        if target_thread != 0 {
            let _ = AttachThreadInput(current, target_thread, BOOL(1));
        }
        let _ = SetForegroundWindow(target);
        let _ = BringWindowToTop(target);
        if target_thread != 0 {
            let _ = AttachThreadInput(current, target_thread, BOOL(0));
        }
        if fg_thread != 0 {
            let _ = AttachThreadInput(current, fg_thread, BOOL(0));
        }
    }
}

/// Attend que `target` devienne la fenêtre au premier plan (borné).
fn wait_for_foreground(target: HWND, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let now = unsafe { GetForegroundWindow() };
        if now.0 == target.0 {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

fn capture_steps() -> Result<CaptureResult, CaptureError> {
    // 1. Fenêtre cible.
    //    - Navi n'est PAS au premier plan (raccourci global) : la fenêtre active EST la
    //      bonne cible — on n'y touche pas (pas de focus volé, pas de fenêtre démaximisée).
    //    - Navi EST au premier plan (bouton/panneau) : la sélection vit dans la dernière
    //      fenêtre non-Navi mémorisée ; il faut la réactiver avant d'envoyer Ctrl+C,
    //      sinon le Ctrl+C part dans le webview Navi et le presse-papiers ne change pas.
    let own_pid = unsafe { GetCurrentProcessId() };
    let foreground = unsafe { GetForegroundWindow() };
    let fg_pid = window_pid(foreground);
    let (hwnd, needs_switch) = if foreground.0.is_null() || fg_pid == own_pid {
        match crate::last_foreground() {
            Some(raw) => {
                let target = HWND(raw);
                if unsafe { IsWindow(target) }.as_bool() {
                    (target, true)
                } else {
                    // Fenêtre mémorisée détruite — ne pas lui envoyer quoi que ce soit.
                    crate::clear_last_foreground();
                    return Err(CaptureError::NoForegroundWindow);
                }
            }
            None => return Err(CaptureError::NoForegroundWindow),
        }
    } else {
        (foreground, false)
    };

    // 2. Si Navi a le focus, réactiver la fenêtre cible et vérifier qu'elle l'a bien eu
    //    (SW_RESTORE uniquement si elle est minimisée — ne pas toucher aux maximisées).
    if needs_switch {
        if unsafe { IsIconic(hwnd) }.as_bool() {
            let _ = unsafe { ShowWindow(hwnd, SW_RESTORE) };
        }
        force_foreground(hwnd);
        if !wait_for_foreground(hwnd, Duration::from_millis(800)) {
            return Err(CaptureError::WindowSwitchFailed(
                "Windows a refusé de redonner l'avant-plan à la fenêtre mémorisée".to_string(),
            ));
        }
    }

    // 3. Attendre que Win/Alt/Maj/Ctrl soient relâchés (le raccourci Navi vient
    //     d'être pressé — sinon la cible reçoit « Win+Alt+Ctrl+C », pas « Ctrl+C »).
    wait_for_modifiers_released(Duration::from_millis(1000));

    // 4. Mémoriser le numéro de séquence du presse-papiers avant la copie
    let seq_before = unsafe { GetClipboardSequenceNumber() };

    // 5. Simuler Ctrl+C via SendInput
    send_copy_keystroke()?;

    // 6. Copie asynchrone : attendre que le presse-papiers change
    if !wait_for_clipboard_change(seq_before, CLIPBOARD_CHANGE_TIMEOUT, || unsafe {
        GetClipboardSequenceNumber()
    }) {
        return Err(CaptureError::NoSelection);
    }

    // 7. Ouvrir le presse-papiers (retries : peut être verrouillé par une autre app)
    if !open_clipboard() {
        return Err(CaptureError::ClipboardBusy {
            attempts: OPEN_CLIPBOARD_ATTEMPTS,
        });
    }

    // 8. Lire le contenu (priorité : Unicode > HTML > RTF > ANSI)
    let result = read_clipboard_content();

    // 9. Fermer le presse-papiers (toujours)
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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        String::from_utf16(&utf16).map_err(|e| e.to_string())
    } else {
        Ok(String::from_utf8_lossy(slice)
            .trim_end_matches('\0')
            .to_string())
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
        assert!(success.restore.is_none());

        let error = CaptureResult::failure(CaptureError::NoSelection);
        assert!(!error.success);
        assert!(error.error.unwrap().contains("Aucune sélection"));
        assert!(error.restore.is_none());
    }

    #[test]
    fn messages_derreur_clairs_pour_chaque_cas() {
        let cas = [
            CaptureError::NoForegroundWindow,
            CaptureError::InjectionFailed { sent: 0 },
            CaptureError::NoSelection,
            CaptureError::ClipboardBusy { attempts: 5 },
            CaptureError::NoTextContent,
            CaptureError::ReadFailure("échec GlobalLock".to_string()),
            CaptureError::SnapshotFailed("presse-papiers verrouillé".to_string()),
            CaptureError::WindowSwitchFailed("fenêtre refusée".to_string()),
        ];
        for err in cas {
            let msg = err.to_string();
            assert!(msg.len() > 10, "message trop court pour {err:?}");
            assert!(!msg.starts_with("Error"), "message non clair pour {err:?}");
        }
    }

    #[test]
    fn formats_non_copiables_classes_correctement() {
        // Handles GDI / atom / range GDI OBJ : non copiables
        assert!(!is_global_memory_format(0x0002)); // CF_BITMAP
        assert!(!is_global_memory_format(0x0003)); // CF_METAFILEPICT
        assert!(!is_global_memory_format(0x0009)); // CF_PALETTE
        assert!(!is_global_memory_format(0x0010)); // CF_LOCALE
        assert!(!is_global_memory_format(0x0080)); // CF_OWNERDISPLAY
        assert!(!is_global_memory_format(0x0081)); // CF_DSPTEXT
        assert!(!is_global_memory_format(0x0301)); // GDIOBJFIRST range
        assert!(!is_global_memory_format(0x03FF)); // GDIOBJLAST range
        // Formats mémoire globale : copiables
        assert!(is_global_memory_format(CF_UNICODETEXT));
        assert!(is_global_memory_format(CF_TEXT));
        assert!(is_global_memory_format(CF_HTML));
        assert!(is_global_memory_format(CF_RTF));
        assert!(is_global_memory_format(0xC013)); // registered (Chromium) — mémoire copiable
        assert!(is_global_memory_format(0x0007)); // CF_OEMTEXT
    }

    #[test]
    fn rapport_restauration_partielle_formate() {
        let report = RestoreReport {
            snapshot_items: 4,
            skipped: 1,
            restored: 2,
            failed: vec![0xC013, 0x000D],
        };
        let msg = report
            .summary()
            .expect("restauration partielle doit produire un message");
        assert!(msg.contains("2/4"));
        assert!(msg.contains("0xC013"));

        let complet = RestoreReport {
            snapshot_items: 2,
            skipped: 0,
            restored: 2,
            failed: vec![],
        };
        assert!(complet.summary().is_none());
    }

    #[test]
    fn sequence_changee_est_detectee() {
        let mut seq = 100u32;
        let changed = wait_for_clipboard_change(100, Duration::from_millis(200), || {
            let current = seq;
            seq += 1;
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
