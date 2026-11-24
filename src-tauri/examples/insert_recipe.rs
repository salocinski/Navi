//! Recette de compatibilité de la saisie dictée (carte N-096).
//!
//! Donne le focus à la fenêtre cible (compte à rebours), y saisit un texte témoin avec
//! `insert_text` (le même chemin que la dictée) puis, avec `--relecture`, sélectionne tout
//! (Ctrl+A) et relit le contenu par la capture presse-papiers pour le comparer.
//!
//!   cargo run --example insert_recipe -- [--relecture] [secondes]

use navi_lib::capture::capture_selection;
use navi_lib::insert::insert_text;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

const TEMOIN: &str = "Dictée ✓ œuvre « test » 😀\nligne 2";

fn ctrl_a() {
    let touche = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..Default::default()
            },
        },
    };
    let inputs = [
        touche(VK_CONTROL, false),
        touche(VK_A, false),
        touche(VK_A, true),
        touche(VK_CONTROL, true),
    ];
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let relecture = args.iter().any(|a| a == "--relecture");
    let secondes: u64 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(5);
    println!("Saisie dans {secondes} s : donnez le focus à la fenêtre cible.");
    std::thread::sleep(Duration::from_secs(secondes));
    match insert_text(TEMOIN) {
        Ok(n) => println!("SAISIE ok ({n} événements)"),
        Err(e) => {
            println!("SAISIE ECHEC : {e}");
            std::process::exit(2);
        }
    }
    if !relecture {
        return;
    }
    std::thread::sleep(Duration::from_millis(400));
    ctrl_a();
    std::thread::sleep(Duration::from_millis(300));
    let r = capture_selection();
    let lu = r.text.replace("\r\n", "\n");
    if r.success && lu.trim_end() == TEMOIN {
        println!("RELECTURE identique");
    } else {
        println!(
            "RELECTURE DIFFERENTE (succès capture : {}) : {:?}",
            r.success, lu
        );
        std::process::exit(3);
    }
}
