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
pub(crate) mod parametres;

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

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
            "quit" => std::process::exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Clic gauche : bascule l'affichage de la fenêtre principale (panneau A = N-048).
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
                && let Some(window) = tray.app_handle().get_webview_window("main")
            {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
