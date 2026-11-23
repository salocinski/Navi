//! Appui maintenu du raccourci de dictée (carte N-089).
//!
//! `RegisterHotKey` n'émet que l'appui (et le répète tant que la touche reste enfoncée), jamais le
//! relâchement : après l'appui, on interroge `GetAsyncKeyState` sur la touche principale du combo.

use global_hotkey::hotkey::HotKey;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Intervalle d'interrogation de la touche (latence de relâchement imperceptible, coût CPU nul).
pub const POLL: Duration = Duration::from_millis(15);
/// Plafond d'un appui maintenu (spike N-087 : < 4 s de transcription à 60 s).
pub const MAX_HOLD: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldEnd {
    Released,
    TimedOut,
}

/// Un seul appui actif à la fois : ignore les répétitions de `WM_HOTKEY` pendant le maintien.
#[derive(Default)]
pub struct HoldGuard(AtomicBool);

impl HoldGuard {
    /// `true` si cet appui démarre un nouveau maintien, `false` si un maintien est déjà en cours.
    pub fn try_begin(&self) -> bool {
        !self.0.swap(true, Ordering::SeqCst)
    }

    pub fn end(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Attend que `is_down` retombe, ou que `max` soit écoulé.
pub fn watch_release(mut is_down: impl FnMut() -> bool, poll: Duration, max: Duration) -> HoldEnd {
    let start = Instant::now();
    loop {
        if !is_down() {
            return HoldEnd::Released;
        }
        if start.elapsed() >= max {
            return HoldEnd::TimedOut;
        }
        std::thread::sleep(poll);
    }
}

/// Code de touche virtuelle Windows de la touche principale d'un combo (« Alt+Space » → 0x20).
/// shortcut: espace, lettres et F1-F12 seulement ; étendre si un autre raccourci de dictée est permis (N-093).
pub fn main_key_vk(combo: &str) -> Option<u16> {
    combo.parse::<HotKey>().ok()?; // combo valide
    let key = combo.rsplit('+').next()?.trim().to_ascii_uppercase();
    match key.as_str() {
        "SPACE" => Some(0x20),
        k if k.len() == 1 && k.as_bytes()[0].is_ascii_uppercase() => Some(k.as_bytes()[0] as u16),
        k => match k.strip_prefix('F')?.parse::<u16>() {
            Ok(n @ 1..=12) => Some(0x6F + n),
            _ => None,
        },
    }
}

#[cfg(windows)]
fn key_is_down(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    // Bit de poids fort = touche actuellement enfoncée.
    (unsafe { GetAsyncKeyState(vk as i32) } as u16) & 0x8000 != 0
}

#[cfg(not(windows))]
fn key_is_down(_vk: u16) -> bool {
    false
}

/// Surveille le relâchement de `vk` sur un thread et appelle `on_end` (fin, durée) ; libère `guard` avant.
pub fn spawn_hold_watch(
    vk: u16,
    guard: Arc<HoldGuard>,
    on_end: impl FnOnce(HoldEnd, Duration) + Send + 'static,
) {
    std::thread::spawn(move || {
        let start = Instant::now();
        let end = watch_release(|| key_is_down(vk), POLL, MAX_HOLD);
        guard.end();
        on_end(end, start.elapsed());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relachement_detecte_apres_quelques_interrogations() {
        let mut n = 0;
        let end = watch_release(
            || {
                n += 1;
                n < 4
            },
            Duration::from_millis(1),
            Duration::from_secs(5),
        );
        assert_eq!(end, HoldEnd::Released);
        assert_eq!(n, 4);
    }

    #[test]
    fn plafond_atteint_si_la_touche_reste_enfoncee() {
        let end = watch_release(|| true, Duration::from_millis(1), Duration::from_millis(20));
        assert_eq!(end, HoldEnd::TimedOut);
    }

    #[test]
    fn touche_deja_relachee_termine_immediatement() {
        let end = watch_release(|| false, Duration::from_secs(1), Duration::from_secs(1));
        assert_eq!(end, HoldEnd::Released);
    }

    #[test]
    fn les_repetitions_sont_ignorees_pendant_le_maintien() {
        let g = HoldGuard::default();
        assert!(g.try_begin());
        assert!(!g.try_begin());
        g.end();
        assert!(g.try_begin());
    }

    #[test]
    fn touche_principale_du_combo() {
        assert_eq!(main_key_vk("Alt+Space"), Some(0x20));
        assert_eq!(main_key_vk("Ctrl+Shift+F9"), Some(0x78));
        assert_eq!(main_key_vk("Super+Alt+S"), Some(0x53));
        assert_eq!(main_key_vk("pas un raccourci"), None);
    }
}
