//! Fenêtre flottante du cercle animé (carte N-077, révision explicite de TW-68).
//!
//! Une pastille ronde toujours-au-dessus, sans focus, non cliquable, visible de
//! la combinaison (raccourci → Capture) jusqu'à la fin de l'audio ; rouge en erreur.
//! La fenêtre elle-même n'affiche qu'un SVG animé en CSS (`public/floating.html`,
//! aucun asset, aucun build supplémentaire) ; la visibilité est pilotée par la machine.

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub(crate) const LABEL: &str = "flotteur";
/// N-107 : la fenêtre porte aussi le cartouche de dictée, au-dessus de la pastille.
const WIDTH: i32 = 200;
// shortcut: hauteur fixe (le cartouche grandit vers le haut dans une fenêtre transparente non
// cliquable) ; au-delà d'environ 40 lignes le début du texte est rogné, passer à une fenêtre
// redimensionnée selon le texte si cela arrive en pratique.
const HEIGHT: i32 = 700;
/// Distance du bord droit de la zone de travail au bord gauche de la fenêtre. Le bord gauche du
/// cercle est à 220 px du bord droit (ancien bord + 144 px vers la gauche, puis 72 px vers la
/// droite : demandes humaines 2026-11-24) ; il est centré dans la fenêtre, à 76 px de son bord.
const LEFT_OFFSET: i32 = 296;
/// Le cercle, dessiné 4 px au-dessus du bas de la fenêtre, finit ainsi à 5 px de la barre des tâches.
const BOTTOM_GAP: i32 = 1;

/// Crée la fenêtre flottante (masquée par défaut). Appeler depuis `setup`.
pub(crate) fn create_floating_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let monitor = app.primary_monitor().ok().flatten();
    let mut height = HEIGHT;
    let (x, y) = match &monitor {
        Some(m) => {
            // Bas-droite de la zone de travail (au-dessus de la barre des tâches).
            let wa = m.work_area();
            height = HEIGHT.min(wa.size.height as i32 - BOTTOM_GAP);
            (
                (wa.position.x + wa.size.width as i32 - LEFT_OFFSET) as f64,
                (wa.position.y + wa.size.height as i32 - height - BOTTOM_GAP) as f64,
            )
        }
        // Écran introuvable : coin par défaut (visible mais discret).
        None => (100.0, 100.0),
    };

    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("floating.html".into()))
        .title("Navi — statut")
        .inner_size(WIDTH as f64, height as f64)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        // Ne jamais devenir la fenêtre active : la saisie de la dictée part vers la fenêtre visée.
        .focusable(false)
        .visible(false)
        .build()?;
    // Non cliquable : la pastille ne peut jamais voler un clic à l'app capturée.
    window.set_ignore_cursor_events(true)?;
    Ok(())
}
