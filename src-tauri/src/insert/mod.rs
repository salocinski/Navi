//! Insertion du texte dicté (carte N-091) : saisie Unicode au curseur de la fenêtre au premier plan.
//!
//! `SendInput` + `KEYEVENTF_UNICODE` envoie chaque unité UTF-16 sans dépendre de la disposition du
//! clavier et sans toucher au presse-papiers. La cible est la fenêtre active **au moment de l'envoi**.

use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY, VK_RETURN, VK_TAB,
};

/// Taille d'un lot `SendInput` (événements), pour ne pas saturer la file de la cible.
const BATCH: usize = 256;
/// Attente maximale du relâchement des modificateurs (le raccourci de dictée est encore enfoncé).
const MODIFIERS_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// Unité UTF-16 saisie telle quelle (accents, emoji = deux unités).
    Unit(u16),
    /// Touche virtuelle (Entrée, Tabulation) : certaines applications ignorent l'Unicode pour ces caractères.
    Vk(u16),
}

#[derive(Debug, PartialEq, Eq)]
pub enum InsertError {
    /// Aucun événement accepté : cible protégée (fenêtre élevée alors que Navi ne l'est pas, bureau sécurisé).
    Blocked,
    /// Seule une partie du texte a été injectée.
    Partial { sent: usize, total: usize },
}

impl std::fmt::Display for InsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsertError::Blocked => write!(
                f,
                "Saisie refusée par le système (fenêtre protégée ou exécutée en administrateur)"
            ),
            InsertError::Partial { sent, total } => {
                write!(f, "Saisie interrompue ({sent}/{total} touches injectées)")
            }
        }
    }
}

impl std::error::Error for InsertError {}

/// Décompose le texte en actions clavier. Retours à la ligne → Entrée, tabulation → Tab ;
/// les autres caractères de contrôle sont ignorés.
pub fn plan(text: &str) -> Vec<KeyAction> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push(KeyAction::Vk(VK_RETURN.0));
            }
            '\n' => out.push(KeyAction::Vk(VK_RETURN.0)),
            '\t' => out.push(KeyAction::Vk(VK_TAB.0)),
            c if c.is_control() => {}
            c => {
                let mut buf = [0u16; 2];
                out.extend(c.encode_utf16(&mut buf).iter().map(|u| KeyAction::Unit(*u)));
            }
        }
    }
    out
}

fn key_event(action: KeyAction, up: bool) -> INPUT {
    let (vk, scan, mut flags) = match action {
        KeyAction::Unit(u) => (VIRTUAL_KEY(0), u, KEYEVENTF_UNICODE),
        KeyAction::Vk(v) => (VIRTUAL_KEY(v), 0, KEYBD_EVENT_FLAGS(0)),
    };
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// Appui puis relâchement de chaque action.
pub fn build_inputs(actions: &[KeyAction]) -> Vec<INPUT> {
    actions
        .iter()
        .flat_map(|a| [key_event(*a, false), key_event(*a, true)])
        .collect()
}

/// Saisit `text` dans la fenêtre au premier plan. Rend le nombre d'événements injectés.
pub fn insert_text(text: &str) -> Result<usize, InsertError> {
    let inputs = build_inputs(&plan(text));
    if inputs.is_empty() {
        return Ok(0);
    }
    // Les touches Alt/Win du raccourci de dictée transformeraient la saisie en raccourcis.
    crate::capture::wait_for_modifiers_released(MODIFIERS_TIMEOUT);
    let mut sent = 0usize;
    for batch in inputs.chunks(BATCH) {
        let n = unsafe { SendInput(batch, std::mem::size_of::<INPUT>() as i32) } as usize;
        sent += n;
        if n != batch.len() {
            return Err(if sent == 0 {
                InsertError::Blocked
            } else {
                InsertError::Partial {
                    sent,
                    total: inputs.len(),
                }
            });
        }
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_et_ponctuation_en_unites_simples() {
        assert_eq!(
            plan("é!"),
            vec![KeyAction::Unit(0x00E9), KeyAction::Unit(0x0021)]
        );
    }

    #[test]
    fn emoji_donne_une_paire_de_substitution() {
        assert_eq!(
            plan("😀"),
            vec![KeyAction::Unit(0xD83D), KeyAction::Unit(0xDE00)]
        );
    }

    #[test]
    fn retours_a_la_ligne_et_tabulations_deviennent_des_touches() {
        assert_eq!(
            plan("a\r\nb\nc\td"),
            vec![
                KeyAction::Unit('a' as u16),
                KeyAction::Vk(VK_RETURN.0),
                KeyAction::Unit('b' as u16),
                KeyAction::Vk(VK_RETURN.0),
                KeyAction::Unit('c' as u16),
                KeyAction::Vk(VK_TAB.0),
                KeyAction::Unit('d' as u16),
            ]
        );
    }

    #[test]
    fn caracteres_de_controle_ignores_et_texte_vide() {
        assert_eq!(plan("\u{1b}\u{7}"), vec![]);
        assert_eq!(plan(""), vec![]);
        assert_eq!(insert_text(""), Ok(0));
    }

    #[test]
    fn deux_evenements_par_action_avec_les_bons_drapeaux() {
        let inputs = build_inputs(&[KeyAction::Unit(0x00E9), KeyAction::Vk(VK_RETURN.0)]);
        assert_eq!(inputs.len(), 4);
        let ki = |i: usize| unsafe { inputs[i].Anonymous.ki };
        assert_eq!(ki(0).dwFlags, KEYEVENTF_UNICODE);
        assert_eq!(ki(0).wScan, 0x00E9);
        assert_eq!(ki(0).wVk, VIRTUAL_KEY(0));
        assert_eq!(ki(1).dwFlags, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
        assert_eq!(ki(2).dwFlags, KEYBD_EVENT_FLAGS(0));
        assert_eq!(ki(2).wVk, VK_RETURN);
        assert_eq!(ki(3).dwFlags, KEYEVENTF_KEYUP);
    }

    #[test]
    fn erreurs_lisibles() {
        assert!(InsertError::Blocked.to_string().contains("refusée"));
        assert!(
            InsertError::Partial { sent: 2, total: 8 }
                .to_string()
                .contains("2/8")
        );
    }

    /// Vérification réelle : ouvre une fenêtre EDIT, la met au premier plan, y saisit le texte et le relit.
    /// Ne saisit RIEN si la fenêtre n'a pas obtenu le premier plan.
    /// `cargo test saisie_reelle -- --ignored --nocapture` (poste interactif requis).
    #[test]
    #[ignore]
    fn saisie_reelle_dans_une_fenetre_edit() {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::core::w;

        let (tx, rx) = std::sync::mpsc::channel::<isize>();
        let ui = std::thread::spawn(move || unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!(""),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE | WINDOW_STYLE(ES_MULTILINE as u32),
                200,
                200,
                600,
                300,
                None,
                None,
                None,
                None,
            )
            .expect("fenêtre");
            let _ = SetForegroundWindow(hwnd);
            tx.send(hwnd.0 as isize).unwrap();
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
        let raw = rx.recv().unwrap();
        let hwnd = HWND(raw as *mut _);
        std::thread::sleep(Duration::from_millis(600));
        assert_eq!(
            unsafe { GetForegroundWindow() },
            hwnd,
            "la fenêtre de test n'a pas le premier plan : rien n'est saisi"
        );

        let text = "Dictée ✓ œuvre 😀\nligne 2";
        assert!(insert_text(text).expect("saisie") > 0);
        std::thread::sleep(Duration::from_millis(300));

        let mut buf = [0u16; 256];
        let n = unsafe {
            SendMessageW(
                hwnd,
                WM_GETTEXT,
                WPARAM(buf.len()),
                LPARAM(buf.as_mut_ptr() as isize),
            )
        };
        let got = String::from_utf16_lossy(&buf[..n.0 as usize]);
        unsafe {
            let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            let _ = PostMessageW(hwnd, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        let _ = ui.join();
        assert_eq!(got.replace("\r\n", "\n"), text);
    }
}
