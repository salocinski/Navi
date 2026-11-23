//! Lancement au démarrage de Windows (cartes N-054, N-095).
//!
//! Navi s'exécute toujours en administrateur (manifeste, N-095) : une valeur `Run` ne peut pas lancer
//! une application élevée. Le démarrage passe donc par une tâche planifiée à l'ouverture de session,
//! avec privilèges maximaux (aucune invite UAC). L'ancienne valeur `Run` (N-054) est migrée puis retirée.
//!
//! Sécurité : la tâche lance l'exécutable sans invite ; il doit donc être dans `Program Files`
//! (non modifiable par un utilisateur standard), sinon l'activation est refusée en release.

use std::ffi::OsString;
use std::io::Write;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::UI::Shell::{FOLDERID_ProgramFiles, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
use windows::core::HSTRING;

const TASK_NAME: &str = "Navi";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Navi";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const RUN_LEVEL_ADMIN: &str = "HighestAvailable";

/// Active ou désactive le lancement au démarrage (exécutable courant).
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| format!("exécutable introuvable : {e}"))?;
        if !cfg!(debug_assertions) && !in_program_files(&exe) {
            return Err(
                "Le démarrage automatique en administrateur exige une installation dans Program Files"
                    .into(),
            );
        }
        create_task(TASK_NAME, &exe, RUN_LEVEL_ADMIN)?;
        delete_value(RUN_KEY, VALUE_NAME)
    } else {
        if task_exists(TASK_NAME) {
            delete_task(TASK_NAME)?;
        }
        delete_value(RUN_KEY, VALUE_NAME)
    }
}

/// Vrai si Navi est déclaré au démarrage (tâche planifiée, ou ancienne valeur `Run` à migrer).
pub fn is_enabled() -> bool {
    task_exists(TASK_NAME) || value_exists(RUN_KEY, VALUE_NAME)
}

/// Migre l'ancienne valeur `Run` (N-054) vers la tâche planifiée. Sans effet si elle est absente ;
/// en cas d'échec (par exemple processus non élevé) l'ancienne valeur est conservée.
pub fn migrate_legacy() -> Result<(), String> {
    if value_exists(RUN_KEY, VALUE_NAME) {
        set_enabled(true)?;
    }
    Ok(())
}

fn system_dir() -> Result<PathBuf, String> {
    let mut buf = [0u16; 260];
    let n = unsafe { GetSystemDirectoryW(Some(&mut buf)) } as usize;
    if n == 0 || n >= buf.len() {
        return Err("dossier système introuvable".into());
    }
    Ok(PathBuf::from(OsString::from_wide(&buf[..n])))
}

/// `C:\Windows\Temp` : création ouverte aux utilisateurs, mais un fichier créé par un administrateur
/// n'est pas modifiable par eux (le fichier XML d'une tâche élevée ne doit pas pouvoir être substitué).
fn windows_temp() -> Result<PathBuf, String> {
    let sys = system_dir()?;
    Ok(sys
        .parent()
        .ok_or("dossier Windows introuvable")?
        .join("Temp"))
}

fn in_program_files(exe: &Path) -> bool {
    let pf = unsafe {
        let Ok(p) =
            SHGetKnownFolderPath(&FOLDERID_ProgramFiles, KF_FLAG_DEFAULT, HANDLE::default())
        else {
            return false;
        };
        let s = p.to_string();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    };
    let Ok(pf) = pf else { return false };
    let exe = exe.to_string_lossy().to_lowercase();
    let exe = exe.trim_start_matches(r"\\?\");
    exe.starts_with(&format!("{}\\", pf.to_lowercase().trim_end_matches('\\')))
}

fn schtasks(args: &[&str]) -> Result<std::process::Output, String> {
    // Chemin absolu : jamais de résolution par PATH ou répertoire courant depuis un processus élevé.
    std::process::Command::new(system_dir()?.join("schtasks.exe"))
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("schtasks indisponible : {e}"))
}

fn task_exists(name: &str) -> bool {
    schtasks(&["/Query", "/TN", name]).is_ok_and(|o| o.status.success())
}

fn delete_task(name: &str) -> Result<(), String> {
    let out = schtasks(&["/Delete", "/TN", name, "/F"])?;
    check(out, "suppression de la tâche planifiée")
}

fn check(out: std::process::Output, what: &str) -> Result<(), String> {
    if out.status.success() {
        return Ok(());
    }
    Err(format!(
        "{what} impossible : {}",
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

fn create_task(name: &str, exe: &Path, run_level: &str) -> Result<(), String> {
    let xml = task_xml(exe, &current_user(), run_level);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp = windows_temp()?.join(format!("navi-task-{}-{nanos}.xml", std::process::id()));
    // create_new : jamais un fichier préparé par un tiers.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| format!("{} : {e}", tmp.display()))?;
    let written = file.write_all(&utf16_with_bom(&xml));
    drop(file);
    let result = written
        .map_err(|e| e.to_string())
        .and_then(|()| {
            let path = tmp.to_str().ok_or("chemin non UTF-8")?;
            schtasks(&["/Create", "/TN", name, "/XML", path, "/F"])
        })
        .and_then(|out| check(out, "création de la tâche planifiée"));
    let _ = std::fs::remove_file(&tmp);
    result
}

fn current_user() -> String {
    let name = std::env::var("USERNAME").unwrap_or_default();
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{name}"),
        _ => name,
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn utf16_with_bom(s: &str) -> Vec<u8> {
    std::iter::once(0xFEFF_u16)
        .chain(s.encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// Définition de la tâche : à l'ouverture de session de `user`, sans limite de durée, une seule instance.
fn task_xml(exe: &Path, user: &str, run_level: &str) -> String {
    let user = xml_escape(user);
    let command = xml_escape(&exe.to_string_lossy());
    let dir = xml_escape(&exe.parent().unwrap_or(Path::new("")).to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Lance Navi à l'ouverture de session (administrateur).</Description></RegistrationInfo>
  <Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{user}</UserId></LogonTrigger></Triggers>
  <Principals><Principal id="Author"><UserId>{user}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>{run_level}</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author"><Exec><Command>{command}</Command><WorkingDirectory>{dir}</WorkingDirectory></Exec></Actions>
</Task>
"#
    )
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
    use windows::Win32::System::Registry::{
        REG_OPTION_NON_VOLATILE, REG_SZ, RegCreateKeyExW, RegSetValueExW,
    };

    /// Clé et valeur de test : jamais la vraie clé `Run`, nettoyées en fin de test.
    const TEST_KEY: &str = r"Software\Navi\TestAutostart";

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
            .map_err(|e| e.to_string())?;
            let result = RegSetValueExW(key, &HSTRING::from(name), 0, REG_SZ, Some(&bytes));
            let _ = RegCloseKey(key);
            result.ok().map_err(|e| e.to_string())
        }
    }

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

    #[test]
    fn xml_de_tache_eleve_a_l_ouverture_de_session() {
        let xml = task_xml(
            Path::new(r"C:\Program Files\Navi & Cie\navi.exe"),
            r"PC\Jean <test>",
            RUN_LEVEL_ADMIN,
        );
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
        assert!(xml.contains("<LogonTrigger>"));
        assert!(xml.contains("<Command>C:\\Program Files\\Navi &amp; Cie\\navi.exe</Command>"));
        assert!(
            xml.contains("<WorkingDirectory>C:\\Program Files\\Navi &amp; Cie</WorkingDirectory>")
        );
        assert!(xml.contains("PC\\Jean &lt;test&gt;"));
        assert!(!xml.contains("<test>"));
    }

    #[test]
    fn echappement_xml_et_octets_utf16() {
        assert_eq!(
            xml_escape(r#"a&b<c>"d"'e'"#),
            "a&amp;b&lt;c&gt;&quot;d&quot;&apos;e&apos;"
        );
        assert_eq!(utf16_with_bom("é"), vec![0xFF, 0xFE, 0xE9, 0x00]);
    }

    #[test]
    fn program_files_reconnu_sans_confusion_de_prefixe() {
        assert!(in_program_files(Path::new(
            r"C:\Program Files\Navi\navi.exe"
        )));
        assert!(in_program_files(Path::new(
            r"c:\program files\navi\navi.exe"
        )));
        assert!(!in_program_files(Path::new(
            r"C:\Program Files Evil\navi.exe"
        )));
        assert!(!in_program_files(Path::new(
            r"C:\Users\x\AppData\Local\Navi\navi.exe"
        )));
    }

    /// Cycle réel avec une tâche de test sans privilèges (la création élevée exige une session administrateur).
    /// `cargo test tache_reelle -- --ignored`
    #[test]
    #[ignore]
    fn tache_reelle_creation_lecture_suppression() {
        let name = "Navi-test-N095";
        let _ = delete_task(name);
        let exe = std::env::current_exe().unwrap();
        create_task(name, &exe, "LeastPrivilege").unwrap();
        assert!(task_exists(name));
        delete_task(name).unwrap();
        assert!(!task_exists(name));
    }
}
