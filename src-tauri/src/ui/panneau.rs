//! Panneau ancré à l'icône de la zone de notification (carte N-048, maquette A).
//!
//! C'est la fenêtre `main` (frameless, toujours au-dessus, hors barre des tâches) : un clic
//! gauche sur l'icône l'ouvre à côté de l'icône, quel que soit le bord où se trouve la barre
//! des tâches ; elle se referme à la perte de focus (clic extérieur) ou sur Échap.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WindowEvent};

pub(crate) const LABEL: &str = "main";
/// Taille logique du panneau (indépendante du DPI).
const WIDTH: f64 = 340.0;
const HEIGHT: f64 = 440.0;
/// Écart entre l'icône et le panneau, en pixels physiques.
const GAP: i32 = 8;
/// Le clic sur l'icône retire d'abord le focus au panneau (qui se masque) : ce délai évite
/// que ce même clic le rouvre aussitôt.
const REOPEN_GUARD_MS: u64 = 300;

static LAST_BLUR_HIDE_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Rectangle en pixels physiques.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Area {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Bord occupé par la barre des tâches : là où la zone de travail est plus petite que l'écran.
/// Barre masquée automatiquement (zone de travail = écran) : bord le plus proche de l'icône.
fn taskbar_edge(monitor: Area, work: Area, icon: Area) -> Edge {
    if work.y > monitor.y {
        Edge::Top
    } else if work.y + work.h < monitor.y + monitor.h {
        Edge::Bottom
    } else if work.x > monitor.x {
        Edge::Left
    } else if work.x + work.w < monitor.x + monitor.w {
        Edge::Right
    } else {
        let candidates = [
            (icon.y - monitor.y, Edge::Top),
            (monitor.y + monitor.h - (icon.y + icon.h), Edge::Bottom),
            (icon.x - monitor.x, Edge::Left),
            (monitor.x + monitor.w - (icon.x + icon.w), Edge::Right),
        ];
        candidates
            .iter()
            .min_by_key(|(d, _)| *d)
            .map(|(_, e)| *e)
            .unwrap_or(Edge::Bottom)
    }
}

/// Position (coin haut-gauche) d'un panneau `w`×`h` à côté de `icon`, ramenée dans `work`.
pub(crate) fn anchor(monitor: Area, work: Area, icon: Area, w: i32, h: i32) -> (i32, i32) {
    let cx = icon.x + icon.w / 2;
    let cy = icon.y + icon.h / 2;
    let (x, y) = match taskbar_edge(monitor, work, icon) {
        Edge::Bottom => (cx - w / 2, icon.y - GAP - h),
        Edge::Top => (cx - w / 2, icon.y + icon.h + GAP),
        Edge::Left => (icon.x + icon.w + GAP, cy - h / 2),
        Edge::Right => (icon.x - GAP - w, cy - h / 2),
    };
    // Plus grand que la zone de travail : on colle au bord haut/gauche plutôt que de paniquer.
    (
        x.clamp(work.x, (work.x + work.w - w).max(work.x)),
        y.clamp(work.y, (work.y + work.h - h).max(work.y)),
    )
}

/// Branche la fermeture à la perte de focus (appeler depuis `setup`).
pub(crate) fn setup(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        log::warn!("panneau : fenêtre principale introuvable");
        return;
    };
    let handle = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event
            && handle.is_visible().unwrap_or(false)
            && handle.hide().is_ok()
        {
            LAST_BLUR_HIDE_MS.store(now_ms(), Ordering::Relaxed);
        }
    });
}

/// Ouvre le panneau à côté de l'icône (`icon` : rectangle de l'icône), ou le ferme s'il est ouvert.
pub(crate) fn toggle(app: &AppHandle, icon: tauri::Rect) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    if now_ms().saturating_sub(LAST_BLUR_HIDE_MS.load(Ordering::Relaxed)) < REOPEN_GUARD_MS {
        return;
    }

    let scale = window.scale_factor().unwrap_or(1.0);
    let pos = icon.position.to_physical::<i32>(scale);
    let size = icon.size.to_physical::<u32>(scale);
    let icon_area = Area {
        x: pos.x,
        y: pos.y,
        w: size.width as i32,
        h: size.height as i32,
    };
    let monitor = app
        .monitor_from_point(
            (pos.x + icon_area.w / 2) as f64,
            (pos.y + icon_area.h / 2) as f64,
        )
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        // Écran introuvable : on affiche le panneau où il se trouve.
        let _ = window.show();
        let _ = window.set_focus();
        return;
    };

    let m_scale = monitor.scale_factor();
    let (w, h) = (
        (WIDTH * m_scale).round() as i32,
        (HEIGHT * m_scale).round() as i32,
    );
    let to_area = |p: &PhysicalPosition<i32>, s: &PhysicalSize<u32>| Area {
        x: p.x,
        y: p.y,
        w: s.width as i32,
        h: s.height as i32,
    };
    let work = monitor.work_area();
    let (x, y) = anchor(
        to_area(monitor.position(), monitor.size()),
        to_area(&work.position, &work.size),
        icon_area,
        w,
        h,
    );
    let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    const MON: Area = Area {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    };
    const W: i32 = 340;
    const H: i32 = 440;

    fn icon(x: i32, y: i32) -> Area {
        Area { x, y, w: 24, h: 24 }
    }

    #[test]
    fn barre_en_bas_panneau_au_dessus() {
        let work = Area { h: 1040, ..MON };
        let (x, y) = anchor(MON, work, icon(1700, 1050), W, H);
        // Le panneau reste dans la zone de travail : il ne recouvre pas la barre des tâches.
        assert_eq!(y, work.y + work.h - H);
        assert_eq!(x, 1712 - W / 2);
    }

    #[test]
    fn barre_en_haut_panneau_en_dessous() {
        let work = Area {
            y: 40,
            h: 1040,
            ..MON
        };
        let (_, y) = anchor(MON, work, icon(1700, 8), W, H);
        assert_eq!(y, 8 + 24 + GAP);
    }

    #[test]
    fn barre_a_gauche_panneau_a_droite_de_l_icone() {
        let work = Area {
            x: 60,
            w: 1860,
            ..MON
        };
        let (x, y) = anchor(MON, work, icon(18, 900), W, H);
        assert_eq!(x, work.x);
        assert_eq!(y, work.y + work.h - H); // icône proche du bas : clampé
    }

    #[test]
    fn barre_a_droite_panneau_a_gauche_de_l_icone() {
        let work = Area { w: 1860, ..MON };
        let (x, _) = anchor(MON, work, icon(1880, 900), W, H);
        assert_eq!(x, work.x + work.w - W);
    }

    #[test]
    fn barre_masquee_automatiquement_bord_le_plus_proche() {
        let (_, y) = anchor(MON, MON, icon(1700, 1060), W, H);
        assert_eq!(y, 1060 - GAP - H);
        let (x, _) = anchor(MON, MON, icon(2, 500), W, H);
        assert_eq!(x, 2 + 24 + GAP);
    }

    #[test]
    fn panneau_ramene_dans_la_zone_de_travail() {
        let work = Area { h: 1040, ..MON };
        let (x, _) = anchor(MON, work, icon(1900, 1050), W, H);
        assert_eq!(x, 1920 - W);
        // Second écran à gauche de l'écran principal (coordonnées négatives).
        let mon2 = Area { x: -1920, ..MON };
        let work2 = Area {
            x: -1920,
            h: 1040,
            ..MON
        };
        let (x2, _) = anchor(mon2, work2, icon(-1915, 1050), W, H);
        assert_eq!(x2, -1920);
    }

    #[test]
    fn panneau_plus_grand_que_la_zone_ne_panique_pas() {
        let small = Area {
            x: 0,
            y: 0,
            w: 200,
            h: 200,
        };
        assert_eq!(anchor(small, small, icon(100, 180), W, H), (0, 0));
    }
}
