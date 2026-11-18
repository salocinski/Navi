//! Vérification des raccourcis candidats Navi (carte TW-018).
//!
//! Enregistre chaque combinaison proposée, affiche succès ou échec, puis sort.
//! La sortie du processus libère les hotkeys (Windows désenregistre à la fin du thread).
//!
//! Usage : cargo run --example hotkey_check

use global_hotkey::{
    GlobalHotKeyManager, hotkey::{HotKey, Code, Modifiers},
};

fn main() {
    let manager = GlobalHotKeyManager::new().expect("GlobalHotKeyManager");

    let candidates: &[(&str, Modifiers, Code)] = &[
        ("Ctrl+Shift+R (lire)", Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyR),
        ("Ctrl+Shift+P (pause)", Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyP),
        ("Ctrl+Shift+S (arrêt)", Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyS),
        ("Ctrl+Alt+Espace (lire)", Modifiers::CONTROL | Modifiers::ALT, Code::Space),
        ("Ctrl+Alt+P (pause)", Modifiers::CONTROL | Modifiers::ALT, Code::KeyP),
        ("Ctrl+Alt+S (arrêt)", Modifiers::CONTROL | Modifiers::ALT, Code::KeyS),
    ];

    let mut ok = 0;
    let mut ko = 0;
    for (label, modifiers, code) in candidates {
        let hotkey = HotKey::new(Some(*modifiers), *code);
        match manager.register(hotkey) {
            Ok(()) => {
                println!("OK    {label}");
                ok += 1;
            }
            Err(err) => {
                println!("ECHEC {label} — {err}");
                ko += 1;
            }
        }
    }

    println!("\nRésumé : {ok} enregistré(s), {ko} en échec.");
    println!("(Les combinaisons en échec sont déjà réservées par l'OS ou une application.)");
}
