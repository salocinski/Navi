//! Fenêtre flottante du cercle animé (carte N-077, révision explicite de TW-68).
//!
//! Une pastille ronde toujours-au-dessus, sans focus, non cliquable, visible de
//! la combinaison (raccourci → Capture) jusqu'à la fin de l'audio ; rouge en erreur.
//! La fenêtre elle-même n'affiche qu'un SVG animé en CSS (`public/floating.html`,
//! aucun asset, aucun build supplémentaire) ; la visibilité est pilotée par la machine.

use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub(crate) const LABEL: &str = "flotteur";
const SIZE: f64 = 72.0;
const MARGIN: i32 = 16;

/// Crée la fenêtre flottante (masquée par défaut). Appeler depuis `setup`.
pub(crate) fn create_floating_window(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let monitor = app.primary_monitor().ok().flatten();
    let (x, y) = match &monitor {
        Some(m) => {
            // Bas-droite de la zone de travail (au-dessus de la barre des tâches).
            let wa = m.work_area();
            (
                (wa.position.x + wa.size.width as i32 - SIZE as i32 - MARGIN) as f64,
                (wa.position.y + wa.size.height as i32 - SIZE as i32 - MARGIN) as f64,
            )
        }
        // Écran introuvable : coin par défaut (visible mais discret).
        None => (100.0, 100.0),
    };

    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("floating.html".into()))
        .title("Navi — statut")
        .inner_size(SIZE, SIZE)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()?;
    // Non cliquable : la pastille ne peut jamais voler un clic à l'app capturée.
    window.set_ignore_cursor_events(true)?;
    Ok(())
}
