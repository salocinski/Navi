//! Interface utilisateur : icône de la zone de notification et menu (carte N-047).
//!
//! Conforme à la DA (`docs/pistes-interface.html`) : icône ronde sphère néon, menu
//! ancré à la notification avec Lire la sélection / Pause-Reprise / Arrêter /
//! Paramètres (désactivé — livré par N-051) / Quitter Navi.
//!
//! - Clic gauche : bascule l'affichage de la fenêtre principale.
//! - Clic droit : menu.
//! - Raccourcis réels : Win+Alt+S (lire), Ctrl+Maj+P (pause/reprise), Ctrl+Maj+S (arrêt) —
//!   les libellés du mockup (Ctrl Alt Espace) sont obsolètes (TW-072).
//! - « Démarrer avec Windows » n'est pas encore câblé (N-054) — pas de faux item.

pub(crate) mod flotteur;
pub(crate) mod panneau;
pub(crate) mod parametres;

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

/// Ouvre la fenêtre Paramètres depuis le panneau.
///
/// `async` : une commande synchrone s'exécute sur le thread principal, et créer une
/// fenêtre depuis ce thread bloque sous Windows (fenêtre vide, application figée).
#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    parametres::open(&app).map_err(|e| e.to_string())
}

/// Ferme le panneau (Échap).
#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    if let Some(window) = app.get_webview_window(panneau::LABEL) {
        let _ = window.hide();
    }
}

/// Quitte Navi proprement : libère le moteur et l'audio, puis laisse Tauri fermer ses
/// fenêtres (un `process::exit` brutal avec onnxruntime chargé fait planter la fermeture).
pub(crate) fn quit(app: &AppHandle) {
    if let Some(state) = app.try_state::<crate::NaviState>() {
        // try_lock : ne jamais rester bloqué derrière une génération en cours.
        if let Ok(mut engine) = state.engine.try_lock() {
            *engine = None;
        }
        if let Ok(mut audio) = state.audio.try_lock() {
            *audio = None;
        }
    }
    app.exit(0);
}

/// Quitte Navi (même effet que l'item « Quitter Navi » du menu).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    quit(&app);
}

pub(crate) mod tray_icons;

/// Crée le menu et l'icône de la zone de notification (appeler depuis `setup`).
pub fn create_system_tray(
    app: &AppHandle,
    view: crate::NaviRef,
) -> Result<(), Box<dyn std::error::Error>> {
    let read_item = MenuItemBuilder::new("Lire la sélection")
        .id("read")
        .enabled(true)
        .build(app)?;
    let pause_item = MenuItemBuilder::new("Pause / Reprise")
        .id("pause")
        .enabled(true)
        .build(app)?;
    let stop_item = MenuItemBuilder::new("Arrêter")
        .id("stop")
        .enabled(true)
        .build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let settings_item = MenuItemBuilder::new("Paramètres")
        .id("settings")
        .enabled(true)
        .build(app)?;
    let quit_item = MenuItemBuilder::new("Quitter Navi")
        .id("quit")
        .enabled(true)
        .build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[
            &read_item,
            &pause_item,
            &stop_item,
            &separator,
            &settings_item,
            &separator,
            &quit_item,
        ])
        .build()?;

    let icon = tauri::image::Image::from_bytes(tray_icons::READY_PNG)
        .map_err(|err| format!("frame d'icône embarquée illisible : {err}"))?;

    TrayIconBuilder::with_id(tray_icons::TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Navi — Lecteur de texte à voix haute (local)")
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "read" => {
                let phase = view.machine.state();
                let busy = matches!(
                    phase,
                    crate::state::PlaybackState::Capturing
                        | crate::state::PlaybackState::Generating
                        | crate::state::PlaybackState::Playing
                        | crate::state::PlaybackState::Paused
                );
                if busy {
                    eprintln!("[navi] tray — lecture déjà en cours, ignoré");
                    return;
                }
                if let Err(err) = crate::read_selection_impl(&view) {
                    view.machine.set_error(err.clone());
                    eprintln!("[navi] tray — lecture échouée : {err}");
                }
            }
            "pause" => {
                let outcome = match view.machine.state() {
                    crate::state::PlaybackState::Paused => crate::resume_reading_impl(&view),
                    _ => crate::pause_reading_impl(&view),
                };
                if let Err(err) = outcome {
                    view.machine.set_error(err.clone());
                    eprintln!("[navi] tray — pause/reprise échouée : {err}");
                }
            }
            "stop" => {
                let _ = crate::stop_reading_impl(&view);
            }
            "settings" => {
                if let Err(err) = parametres::open(app) {
                    eprintln!("[navi] tray — fenêtre Paramètres impossible : {err}");
                }
            }
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Clic gauche : ouvre/ferme le panneau ancré à l'icône (maquette A, N-048).
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                panneau::toggle(tray.app_handle(), rect);
            }
        })
        .build(app)?;

    Ok(())
}
