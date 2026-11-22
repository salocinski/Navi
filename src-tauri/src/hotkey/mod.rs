//! Gestion des raccourcis globaux

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

/// Actions possibles via raccourcis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    ReadSelection,
    PauseResume,
    Stop,
}

impl HotkeyAction {
    pub fn label(&self) -> &'static str {
        match self {
            HotkeyAction::ReadSelection => "Lire la sélection",
            HotkeyAction::PauseResume => "Pause / Reprise",
            HotkeyAction::Stop => "Arrêt",
        }
    }
}

/// Libellé lisible d'une combinaison (pour l'UI et les rapports).
pub fn combo_label(modifiers: Modifiers, key: Code) -> String {
    let mut parts = Vec::new();
    if modifiers.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if modifiers.contains(Modifiers::SHIFT) {
        parts.push("Maj");
    }
    if modifiers.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if modifiers.contains(Modifiers::SUPER) {
        parts.push("Win");
    }
    parts.push(match key {
        Code::KeyR => "R",
        Code::KeyP => "P",
        Code::KeyS => "S",
        Code::Space => "Espace",
        Code::Escape => "Échap",
        _ => "?",
    });
    parts.join("+")
}

/// Rapport d'enregistrement des raccourcis par défaut :
/// succès partiel toléré, chaque échec porte un message clair.
#[derive(Debug, Default)]
pub struct RegistrationReport {
    /// (action, combo) enregistrés avec succès.
    pub registered: Vec<(HotkeyAction, String)>,
    /// (action, combo, erreur) non enregistrés.
    pub failed: Vec<(HotkeyAction, String, String)>,
}

impl RegistrationReport {
    /// Message clair si au moins un raccourci est en échec, `None` sinon.
    pub fn failure_message(&self) -> Option<String> {
        if self.failed.is_empty() {
            return None;
        }
        let details = self
            .failed
            .iter()
            .map(|(action, combo, err)| {
                format!("{combo} → {action} : {err}", action = action.label())
            })
            .collect::<Vec<_>>()
            .join(" ; ");
        Some(format!(
            "Raccourci(s) indisponible(s) — déjà réservés par l'OS ou une autre application : {details}"
        ))
    }

    pub fn is_complete_success(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Gestionnaire de raccourcis globaux
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    /// id → action, partagé avec le thread d'écoute (réenregistrement à chaud, carte N-052).
    hotkeys: Arc<Mutex<HashMap<u32, HotkeyAction>>>,
    /// Raccourcis actuellement enregistrés auprès de l'OS (pour les désenregistrer).
    registered: Vec<HotKey>,
    last_action: Arc<Mutex<Option<HotkeyAction>>>,
}

/// Raccourcis (action, combinaison) tels que configurés.
pub fn bindings_of(config: &crate::config::AppConfig) -> Vec<(HotkeyAction, String)> {
    vec![
        (HotkeyAction::ReadSelection, config.hotkey_read.clone()),
        (HotkeyAction::PauseResume, config.hotkey_pause.clone()),
        (HotkeyAction::Stop, config.hotkey_stop.clone()),
    ]
}

impl HotkeyManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;
        Ok(Self {
            manager,
            hotkeys: Arc::new(Mutex::new(HashMap::new())),
            registered: Vec::new(),
            last_action: Arc::new(Mutex::new(None)),
        })
    }

    /// Enregistre un raccourci
    pub fn register(
        &mut self,
        modifiers: Modifiers,
        key: Code,
        action: HotkeyAction,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let hotkey = HotKey::new(Some(modifiers), key);
        let id = hotkey.id();
        self.manager.register(hotkey)?;
        self.registered.push(hotkey);
        self.hotkeys.lock().unwrap().insert(id, action);
        Ok(id)
    }

    /// Map id → action partagée avec le thread d'écoute (suit les réenregistrements).
    pub(crate) fn hotkeys_shared(&self) -> Arc<Mutex<HashMap<u32, HotkeyAction>>> {
        self.hotkeys.clone()
    }

    /// Remplace **tous** les raccourcis par `bindings` (carte N-052).
    ///
    /// Tolérant comme `register_defaults` : chaque combinaison est tentée seule, un échec
    /// (syntaxe, conflit avec l'OS ou une autre application) est rapporté sans bloquer les autres.
    /// À appeler sur le thread qui a créé le gestionnaire (contrainte de `RegisterHotKey`).
    pub fn apply(&mut self, bindings: &[(HotkeyAction, String)]) -> RegistrationReport {
        for hotkey in self.registered.drain(..) {
            let _ = self.manager.unregister(hotkey);
        }
        self.hotkeys.lock().unwrap().clear();
        let mut report = RegistrationReport::default();
        for (action, combo) in bindings {
            let outcome = combo
                .parse::<HotKey>()
                .map_err(|e| e.to_string())
                .and_then(|hotkey| {
                    self.manager.register(hotkey).map_err(|e| e.to_string())?;
                    self.registered.push(hotkey);
                    self.hotkeys.lock().unwrap().insert(hotkey.id(), *action);
                    Ok(())
                });
            match outcome {
                Ok(()) => report.registered.push((*action, combo.clone())),
                Err(err) => report.failed.push((*action, combo.clone(), err)),
            }
        }
        report
    }

    /// Enregistre les raccourcis par défaut Navi ; n'abandonne pas au premier échec.
    ///
    /// Comportement en cas de conflit (carte TW-018) :
    /// - chaque combinaison est tentée indépendamment ;
    /// - un conflit (combinaison déjà réservée) → échec isolé, message clair, les autres
    ///   raccourcis sont enregistrés quand même ;
    /// - le rapport `RegistrationReport` porte le détail pour affichage utilisateur.
    pub fn register_defaults(&mut self) -> Result<RegistrationReport, Box<dyn std::error::Error>> {
        let defaults: &[(Modifiers, Code, HotkeyAction)] = &[
            // Win+Alt+S : lire la sélection — 1 seule main, « S » pour Speak, libre sur la machine (carte TW-072)
            (
                Modifiers::SUPER | Modifiers::ALT,
                Code::KeyS,
                HotkeyAction::ReadSelection,
            ),
            // Ctrl+Maj+P / Ctrl+Maj+S : pause/reprise et arrêt
            (
                Modifiers::CONTROL | Modifiers::SHIFT,
                Code::KeyP,
                HotkeyAction::PauseResume,
            ),
            (
                Modifiers::CONTROL | Modifiers::SHIFT,
                Code::KeyS,
                HotkeyAction::Stop,
            ),
        ];

        let mut report = RegistrationReport::default();
        for (modifiers, key, action) in defaults {
            let label = combo_label(*modifiers, *key);
            match self.register(*modifiers, *key, *action) {
                Ok(_) => report.registered.push((*action, label)),
                Err(err) => report.failed.push((*action, label, err.to_string())),
            }
        }
        Ok(report)
    }

    /// Démarre l'écoute des événements (bloquant)
    pub fn listen(&self) -> Result<(), Box<dyn std::error::Error>> {
        let receiver = GlobalHotKeyEvent::receiver();
        let hotkeys = self.hotkeys.clone();
        let last_action = self.last_action.clone();

        thread::spawn(move || {
            for event in receiver {
                let action = hotkeys.lock().unwrap().get(&event.id).copied();
                if let Some(action) = action {
                    *last_action.lock().unwrap() = Some(action);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combo_label_formatte_lisiblement() {
        assert_eq!(
            combo_label(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyR),
            "Ctrl+Maj+R"
        );
        assert_eq!(
            combo_label(Modifiers::CONTROL | Modifiers::ALT, Code::Space),
            "Ctrl+Alt+Espace"
        );
        assert_eq!(combo_label(Modifiers::ALT, Code::Escape), "Alt+Échap");
    }

    #[test]
    fn rapport_vide_est_un_succes_complet() {
        let report = RegistrationReport::default();
        assert!(report.is_complete_success());
        assert!(report.failure_message().is_none());
    }

    #[test]
    fn rapport_partiel_produit_un_message_clair() {
        let mut report = RegistrationReport::default();
        report
            .registered
            .push((HotkeyAction::ReadSelection, "Ctrl+Maj+R".to_string()));
        report.failed.push((
            HotkeyAction::Stop,
            "Ctrl+Alt+Espace".to_string(),
            "HotKey already registered".to_string(),
        ));
        assert!(!report.is_complete_success());
        let msg = report
            .failure_message()
            .expect("un message clair est attendu");
        assert!(msg.contains("Ctrl+Alt+Espace"));
        assert!(msg.contains("Arrêt"));
        assert!(msg.contains("déjà réservés"));
        assert!(msg.contains("HotKey already registered"));
    }

    #[test]
    fn action_labels() {
        assert_eq!(HotkeyAction::ReadSelection.label(), "Lire la sélection");
        assert_eq!(HotkeyAction::PauseResume.label(), "Pause / Reprise");
        assert_eq!(HotkeyAction::Stop.label(), "Arrêt");
    }

    /// Combinaisons volontairement exotiques : aucune chance de heurter les vrais raccourcis de Navi.
    #[test]
    fn apply_isole_les_echecs_et_remplace_les_raccourcis() {
        let Ok(mut manager) = HotkeyManager::new() else {
            return; // pas de fenêtre cachée possible (session sans bureau) : rien à vérifier
        };
        let report = manager.apply(&[
            (HotkeyAction::ReadSelection, "Ctrl+Alt+Shift+F9".into()),
            (HotkeyAction::PauseResume, "pas un raccourci".into()),
            (HotkeyAction::Stop, "Ctrl+Alt+Shift+F10".into()),
        ]);
        assert_eq!(report.registered.len(), 2);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, HotkeyAction::PauseResume);
        assert_eq!(manager.hotkeys_shared().lock().unwrap().len(), 2);

        // Remplacement : les anciens sont désenregistrés (F9 redevient libre) et la table est mise à jour.
        let report = manager.apply(&[(HotkeyAction::Stop, "Ctrl+Alt+Shift+F9".into())]);
        assert!(report.is_complete_success(), "{:?}", report.failed);
        let map = manager.hotkeys_shared();
        let map = map.lock().unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.values().all(|a| *a == HotkeyAction::Stop));
    }

    #[test]
    fn bindings_suivent_la_config() {
        let b = bindings_of(&crate::config::AppConfig::default());
        assert_eq!(b.len(), 3);
        assert_eq!(
            b[0],
            (HotkeyAction::ReadSelection, "Super+Alt+S".to_string())
        );
    }
}
