//! Lancement au démarrage de Windows (carte N-054).
//!
//! Valeur `Navi` sous `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` : propre à
//! l'utilisateur, sans droits administrateur ni dépendance supplémentaire.

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::HSTRING;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Navi";

/// Active ou désactive le lancement au démarrage (exécutable courant).
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| format!("exécutable introuvable : {e}"))?;
        // Guillemets : le chemin peut contenir des espaces.
        set_value(RUN_KEY, VALUE_NAME, &format!("\"{}\"", exe.display()))
    } else {
        delete_value(RUN_KEY, VALUE_NAME)
    }
}

/// Vrai si Navi est déclaré au démarrage de Windows.
pub fn is_enabled() -> bool {
    value_exists(RUN_KEY, VALUE_NAME)
}

fn set_value(subkey: &str, name: &str, data: &str) -> Result<(), String> {
    let bytes: Vec<u8> = data
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(subkey),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()
        .map_err(|e| format!("registre inaccessible : {e}"))?;
        let result = RegSetValueExW(key, &HSTRING::from(name), 0, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(key);
        result
            .ok()
            .map_err(|e| format!("écriture du registre impossible : {e}"))
    }
}

fn delete_value(subkey: &str, name: &str) -> Result<(), String> {
    let mut key = HKEY::default();
    unsafe {
        let opened = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(subkey),
            0,
            KEY_SET_VALUE,
            &mut key,
        );
        if opened == ERROR_FILE_NOT_FOUND {
            return Ok(()); // clé absente : rien à retirer
        }
        opened
            .ok()
            .map_err(|e| format!("registre inaccessible : {e}"))?;
        let result = RegDeleteValueW(key, &HSTRING::from(name));
        let _ = RegCloseKey(key);
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(()); // déjà retiré
        }
        result
            .ok()
            .map_err(|e| format!("suppression du registre impossible : {e}"))
    }
}

fn value_exists(subkey: &str, name: &str) -> bool {
    let mut key = HKEY::default();
    unsafe {
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(subkey),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        ) != ERROR_SUCCESS
        {
            return false;
        }
        let found =
            RegQueryValueExW(key, &HSTRING::from(name), None, None, None, None) == ERROR_SUCCESS;
        let _ = RegCloseKey(key);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clé et valeur de test : jamais la vraie clé `Run`, nettoyées en fin de test.
    const TEST_KEY: &str = r"Software\Navi\TestAutostart";

    #[test]
    fn ecriture_lecture_suppression() {
        let _ = delete_value(TEST_KEY, "Valeur");
        assert!(!value_exists(TEST_KEY, "Valeur"));
        set_value(TEST_KEY, "Valeur", r#""C:\Program Files\Navi\navi.exe""#).unwrap();
        assert!(value_exists(TEST_KEY, "Valeur"));
        delete_value(TEST_KEY, "Valeur").unwrap();
        assert!(!value_exists(TEST_KEY, "Valeur"));
        // Suppression d'une valeur déjà absente : sans erreur.
        delete_value(TEST_KEY, "Valeur").unwrap();
    }
}
