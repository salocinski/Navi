//! Outil de recette manuelle pour la capture presse-papiers (carte TW-043).
//!
//! Protocole de recette (voir `docs/s1-43-capture-restauration.md`) :
//!
//! 1. Copiez un CONTENU TEMOIN dans une autre app (ex. un texte riche depuis Word).
//! 2. Sélectionnez un texte dans Chromium/Word, SANS le copier (la capture simule Ctrl+C).
//! 3. Lancez :  cargo run --example capture_recipe
//! 4. Pendant le compte à rebours, donnez le focus à la fenêtre cible (alt+tab).
//! 5. Lisez le rapport : texte capturé, format, résultat de restauration.
//! 6. Collez (Ctrl+V) dans n'importe quel éditeur : vous devez retrouver le contenu témoin.
//!
//! Répéter l'étape pour : texte riche, image (CF_DIB), fichier (CF_HDROP),
//! app gelée (WM_COPY traité en > 1200 ms), presse-papiers verrouillé.

use navi_lib::capture::{CaptureResult, capture_selection};
use std::time::Duration;

fn main() {
    println!("=== Navi — recette capture presse-papiers ===\n");
    println!("Sélectionne un texte dans la fenêtre cible, puis donne-lui le focus maintenant.");
    println!("Capture dans :");

    for remaining in (1..=5).rev() {
        println!("  {remaining}…");
        std::thread::sleep(Duration::from_secs(1));
    }
    println!("  capture !\n");

    let CaptureResult {
        text,
        format_used,
        success,
        error,
        restore,
    } = capture_selection();

    if success {
        let preview: String = text.chars().take(200).collect();
        let ellipsis = if text.chars().count() > 200 {
            "…"
        } else {
            ""
        };
        println!("Réussite ({format_used}) :");
        println!("--- début texte (200 premiers caractères) ---");
        println!("{preview}{ellipsis}");
        println!("--- fin texte ---");
        println!("Longueur totale : {} caractères", text.chars().count());
    } else {
        println!("Échec : {}", error.as_deref().unwrap_or("erreur inconnue"));
    }

    match restore {
        None => println!("Restauration du presse-papiers : COMPLÈTE"),
        Some(msg) => println!("Restauration du presse-papiers : PARTIELLE — {msg}"),
    }

    println!(
        "\nVérification : collez maintenant (Ctrl+V) dans un éditeur — vous devez retrouver le CONTENU TÉMOIN (étape 1)."
    );

    if !success {
        std::process::exit(1);
    }
}
