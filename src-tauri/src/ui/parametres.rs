//! Fenêtre Paramètres (carte N-051), ouverte depuis le menu de l'icône de notification.
//!
//! Page statique `public/settings.html` (même approche que la pastille `floating.html` :
//! aucun build supplémentaire) qui lit/écrit la configuration via les commandes
//! `get_config` / `set_config`. La fenêtre est créée à la demande, puis réaffichée.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub(crate) const LABEL: &str = "parametres";

/// Affiche la fenêtre Paramètres (la crée au premier appel).
pub(crate) fn open(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
        .title("Navi — Paramètres")
        .inner_size(480.0, 640.0)
        .min_inner_size(400.0, 480.0)
        .center()
        .build()?;
    Ok(())
}
