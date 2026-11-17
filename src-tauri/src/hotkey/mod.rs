//! Gestion des raccourcis globaux

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, hotkey::{HotKey, Code, Modifiers},
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

static HOTKEY_MANAGER: Lazy<GlobalHotKeyManager> = Lazy::new(|| {
    GlobalHotKeyManager::new().expect("Failed to create GlobalHotKeyManager")
});

/// Actions possibles via raccourcis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    ReadSelection,
    PauseResume,
    Stop,
}

/// Gestionnaire de raccourcis globaux
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkeys: HashMap<u32, HotkeyAction>, // id -> action
    last_action: Arc<Mutex<Option<HotkeyAction>>>,
}

impl HotkeyManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;
        Ok(Self {
            manager,
            hotkeys: HashMap::new(),
            last_action: Arc::new(Mutex::new(None)),
        })
    }
    
    /// Enregistre un raccourci
    pub fn register(&mut self, modifiers: Modifiers, key: Code, action: HotkeyAction) -> Result<u32, Box<dyn std::error::Error>> {
        let hotkey = HotKey::new(Some(modifiers), key);
        let id = hotkey.id();
        self.manager.register(hotkey)?;
        self.hotkeys.insert(id, action);
        Ok(id)
    }
    
    /// Enregistre les raccourcis par défaut Navi
    pub fn register_defaults(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Ctrl+Shift+R : Lire sélection
        self.register(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyR, HotkeyAction::ReadSelection)?;
        
        // Ctrl+Shift+P : Pause / Reprise
        self.register(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyP, HotkeyAction::PauseResume)?;
        
        // Ctrl+Shift+S : Arrêt
        self.register(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyS, HotkeyAction::Stop)?;
        
        Ok(())
    }
    
    /// Démarre l'écoute des événements (bloquant)
    pub fn listen(&self) -> Result<(), Box<dyn std::error::Error>> {
        let receiver = GlobalHotKeyEvent::receiver();
        let hotkeys = self.hotkeys.clone();
        let last_action = self.last_action.clone();
        
        thread::spawn(move || {
            for event in receiver {
                if let Some(action) = hotkeys.get(&event.id) {
                    *last_action.lock().unwrap() = Some(*action);
                }
            }
        });
        
        Ok(())
    }
    
    /// Récupère la dernière action détectée
    pub fn last_action(&self) -> Option<HotkeyAction> {
        *self.last_action.lock().unwrap()
    }
    
    /// Prend la dernière action (la consomme)
    pub fn take_last_action(&self) -> Option<HotkeyAction> {
        let mut lock = self.last_action.lock().unwrap();
        lock.take()
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new().expect("Failed to create HotkeyManager")
    }
}