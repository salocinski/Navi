//! Interface utilisateur : zone de notification, panneau, menu

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WindowBuilder};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

/// Crée le menu de la zone de notification (Tauri 2.x)
pub fn create_system_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let read_item = MenuItemBuilder::new("read")
        .text("Lire la sélection")
        .accelerator("Ctrl+Shift+R")
        .build(app)?;
    let pause_item = MenuItemBuilder::new("pause")
        .text("Pause / Reprise")
        .accelerator("Ctrl+Shift+P")
        .build(app)?;
    let stop_item = MenuItemBuilder::new("stop")
        .text("Arrêt")
        .accelerator("Ctrl+Shift+S")
        .build(app)?;
    let separator = PredefinedMenuItem::Separator::new(app)?;
    let settings_item = MenuItemBuilder::new("settings")
        .text("Paramètres")
        .build(app)?;
    let quit_item = MenuItemBuilder::new("quit")
        .text("Quitter")
        .build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&read_item, &pause_item, &stop_item, &separator, &settings_item, &separator, &quit_item])
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Navi - Lecteur TTS local")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: _, .. } = event {
                // Click simple - pourrait afficher le panneau
            }
        })
        .on_menu_item_event(|app, event| match event.id.as_ref() {
            "read" => { let _ = app.emit("hotkey_read", ()); }
            "pause" => { let _ = app.emit("hotkey_pause", ()); }
            "stop" => { let _ = app.emit("hotkey_stop", ()); }
            "settings" => {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => { std::process::exit(0); }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Gère les événements de la zone de notification
pub fn handle_system_tray_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click { button: _, .. } = event {
        // Click simple - pourrait afficher le panneau
    }
}

/// Configuration de la fenêtre principale (cachée au démarrage)
pub fn setup_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Crée la fenêtre de paramètres
pub fn create_settings_window(app: &AppHandle) -> Result<tauri::WebviewWindow, Box<dyn std::error::Error>> {
    Ok(WindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("settings.html".into()),
    )
    .title("Paramètres Navi")
    .inner_size(400.0, 500.0)
    .resizable(false)
    .maximizable(false)
    .decorations(true)
    .always_on_top(false)
    .build()?)
}

/// Panneau flottant ancré à l'icône de notification
pub struct FloatingPanel {
    window: Option<tauri::WebviewWindow>,
}

impl FloatingPanel {
    pub fn new(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let window = WindowBuilder::new(
            app,
            "panel",
            WebviewUrl::App("panel.html".into()),
        )
        .title("")
        .inner_size(280.0, 320.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
        
        Ok(Self { window: Some(window) })
    }
    
    pub fn show(&self) {
        if let Some(window) = &self.window {
            let _ = window.show();
        }
    }
    
    pub fn hide(&self) {
        if let Some(window) = &self.window {
            let _ = window.hide();
        }
    }
    
    pub fn toggle(&self) {
        if let Some(window) = &self.window {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.show();
            }
        }
    }
}