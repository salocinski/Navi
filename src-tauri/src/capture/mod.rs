//! Capture de texte sélectionné via presse-papiers

use std::thread;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM, HGLOBAL};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SendMessageW};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW,
    IsClipboardFormatAvailable, OpenClipboard,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

const WM_COPY: u32 = 0x0301;
const CF_UNICODETEXT: u32 = 13;
const CF_TEXT: u32 = 1;
const CF_HTML: u32 = 0xC00E;
const CF_RTF: u32 = 0xC00F;

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
    
    pub fn error(err: String) -> Self {
        Self { text: String::new(), format_used: String::new(), success: false, error: Some(err) }
    }
}

/// Capture le texte sélectionné via Ctrl+C et lecture presse-papiers
pub fn capture_selection() -> CaptureResult {
    // 1. Envoyer WM_COPY à la fenêtre active
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return CaptureResult::error("Aucune fenêtre au premier plan".to_string());
    }
    
    let _ = unsafe { SendMessageW(HWND(hwnd.0), WM_COPY, WPARAM(0 as usize), LPARAM(0 as isize)) };
    thread::sleep(std::time::Duration::from_millis(200));
    
    // 2. Ouvrir le presse-papiers
    if unsafe { OpenClipboard(None) }.is_err() {
        return CaptureResult::error("Impossible d'ouvrir le presse-papiers".to_string());
    }
    
    // 3. Lire le contenu (priorité: Unicode > HTML > RTF > ANSI)
    let result = read_clipboard_content();
    
    // 4. Fermer le presse-papiers
    let _ = unsafe { CloseClipboard() };
    
    result
}

fn read_clipboard_content() -> CaptureResult {
    let formats_priority = [
        (13u32, "Unicode"),      // CF_UNICODETEXT
        (0xC00Eu32, "HTML"),     // CF_HTML
        (0xC00Fu32, "RTF"),      // CF_RTF
        (1u32, "ANSI"),          // CF_TEXT
    ];
    
    for (fmt, name) in formats_priority {
        if unsafe { IsClipboardFormatAvailable(fmt) }.is_ok() {
            if let Ok(content) = read_format_raw(fmt) {
                if !content.trim().is_empty() {
                    return CaptureResult::success(content, name.to_string());
                }
            }
        }
    }
    
    CaptureResult::error("Aucun texte trouvé dans le presse-papiers".to_string())
}

fn read_format_raw(format: u32) -> Result<String, Box<dyn std::error::Error>> {
    let hglobal = unsafe { windows::Win32::System::DataExchange::GetClipboardData(format)? };
    let handle = HGLOBAL(hglobal.0);
    if handle.0.is_null() {
        return Err("GetClipboardData returned NULL".into());
    }
    
    let size = unsafe { windows::Win32::System::Memory::GlobalSize(handle) };
    if size == 0 {
        return Err("GlobalSize returned 0".into());
    }
    
    let ptr = unsafe { windows::Win32::System::Memory::GlobalLock(handle) };
    if ptr.is_null() {
        return Err("GlobalLock returned NULL".into());
    }
    
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, size as usize) };
    
    let result = if format == 13 { // CF_UNICODETEXT
        let utf16: Vec<u16> = slice.chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&c| c != 0)
            .collect();
        String::from_utf16(&utf16)?
    } else if format == 1 { // CF_TEXT
        String::from_utf8_lossy(slice).trim_end_matches('\0').to_string()
    } else {
        String::from_utf8_lossy(slice).trim_end_matches('\0').to_string()
    };
    
    unsafe { windows::Win32::System::Memory::GlobalUnlock(handle)? };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_capture_result() {
        let success = CaptureResult::success("test".to_string(), "test".to_string());
        assert!(success.success);
        assert_eq!(success.text, "test");
        
        let error = CaptureResult::error("error".to_string());
        assert!(!error.success);
        assert_eq!(error.error, Some("error".to_string()));
    }
}