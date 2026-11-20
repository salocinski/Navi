//! Anneaux d'état sur l'icône de la zone de notification (carte N-050).
//!
//! Maquette C (décision DA, `docs/pistes-interface.html`) :
//! - prêt        : aucun anneau (sphère néon nue)
//! - génération  : anneau bleu qui tourne (8 frames)
//! - lecture     : anneau cyan qui pulse (3 frames)
//! - pause       : anneau ambre fixe
//! - erreur      : pastille rouge
//!
//! Les frames sont des PNG 32×32 embarquées (constituées depuis le logo rond
//! `src-tauri/icons/icon.png`, voir `docs/s1-50-anneaux-etat.md`).
//! Un fil de fond pousse l'icône quand la machine change de phase ou quand
//! l'animation avance — pas de logique UI dans la machine d'états.
//!
//! `prefers-reduced-motion` : équivalent Windows `SPI_GETCLIENTAREAANIMATION`
//! — animations coupées => trames statiques (arc bleu fixe / anneau cyan fixe).

use std::time::Duration;

use tauri::AppHandle;

use crate::state::PlaybackState;

/// Identifiant du tray (voir `create_system_tray`) — permet `tray_by_id`.
pub(crate) const TRAY_ID: &str = "navi-tray";

pub(crate) const READY_PNG: &[u8] = include_bytes!("../../icons/tray/ready.png");
const GEN_FRAMES: [&[u8]; 8] = [
    include_bytes!("../../icons/tray/gen_0.png"),
    include_bytes!("../../icons/tray/gen_1.png"),
    include_bytes!("../../icons/tray/gen_2.png"),
    include_bytes!("../../icons/tray/gen_3.png"),
    include_bytes!("../../icons/tray/gen_4.png"),
    include_bytes!("../../icons/tray/gen_5.png"),
    include_bytes!("../../icons/tray/gen_6.png"),
    include_bytes!("../../icons/tray/gen_7.png"),
];
const PLAY_FRAMES: [&[u8]; 3] = [
    include_bytes!("../../icons/tray/play_0.png"),
    include_bytes!("../../icons/tray/play_1.png"),
    include_bytes!("../../icons/tray/play_2.png"),
];
const PAUSE_PNG: &[u8] = include_bytes!("../../icons/tray/pause.png");
const ERROR_PNG: &[u8] = include_bytes!("../../icons/tray/error.png");

/// État visuel de l'icône — 5 états, conforme à la DA (maquette C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconState {
    /// Prêt — aucun anneau.
    Ready,
    /// Génération / capture — anneau bleu qui tourne.
    Generating,
    /// Lecture — anneau cyan qui pulse.
    Playing,
    /// Pause — anneau ambre fixe.
    Paused,
    /// Erreur — pastille rouge.
    Error,
}

impl From<PlaybackState> for TrayIconState {
    fn from(state: PlaybackState) -> Self {
        match state {
            // Stopped ramène l'icône au repos (comportement aligné sur « prêt »).
            PlaybackState::Idle | PlaybackState::Stopped => Self::Ready,
            PlaybackState::Capturing | PlaybackState::Generating => Self::Generating,
            PlaybackState::Playing => Self::Playing,
            PlaybackState::Paused => Self::Paused,
            PlaybackState::Error => Self::Error,
        }
    }
}

fn frame_bytes(state: TrayIconState, frame: usize) -> &'static [u8] {
    match state {
        TrayIconState::Ready => READY_PNG,
        TrayIconState::Generating => GEN_FRAMES[frame % GEN_FRAMES.len()],
        TrayIconState::Playing => PLAY_FRAMES[frame % PLAY_FRAMES.len()],
        TrayIconState::Paused => PAUSE_PNG,
        TrayIconState::Error => ERROR_PNG,
    }
}

/// Équivalent Windows de `prefers-reduced-motion`.
///
/// `SPI_GETCLIENTAREAANIMATION` renvoie FALSE quand les animations client
/// sont coupées (Paramètres > Accessibilité > Effets visuels).
#[cfg(windows)]
fn reduced_motion() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };
    let mut value: i32 = 1;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut value as *mut i32 as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_ok() && value == 0
}

#[cfg(not(windows))]
fn reduced_motion() -> bool {
    false
}

/// Démarre le fil qui maintient l'icône tray en phase avec la machine.
///
/// Ne redessine que lorsque (état, frame) change — pas de scintillement,
/// pas de set_icon inutile. Tourne pendant toute la vie de l'app ; aucune
/// écriture sur la machine d'états.
pub(crate) fn spawn_icon_animator(app: AppHandle, view: crate::NaviRef) {
    std::thread::spawn(move || {
        let reduced = reduced_motion();
        let mut tick: u64 = 0;
        let mut applied: Option<(TrayIconState, usize)> = None;
        loop {
            let state = TrayIconState::from(view.machine.state());
            let frame = if reduced {
                match state {
                    TrayIconState::Generating => 0, // arc bleu fixe
                    TrayIconState::Playing => 1,    // cyan fixe, opacité moyenne
                    _ => 0,
                }
            } else {
                match state {
                    // ~110 ms par frame ; rotation complète ≈ 0,9 s ; pulsation ≈ 660 ms.
                    TrayIconState::Generating => (tick % 8) as usize,
                    TrayIconState::Playing => ((tick / 2) % 3) as usize,
                    _ => 0,
                }
            };
            if applied != Some((state, frame)) {
                if let Some(tray) = app.tray_by_id(TRAY_ID) {
                    match tauri::image::Image::from_bytes(frame_bytes(state, frame)) {
                        Ok(image) => {
                            if let Err(err) = tray.set_icon(Some(image)) {
                                eprintln!("[navi] tray — mise à jour de l'icône échouée : {err}");
                            }
                        }
                        Err(err) => {
                            eprintln!("[navi] tray — frame embarquée illisible : {err}");
                        }
                    }
                }
                applied = Some((state, frame));
            }
            tick += 1;
            std::thread::sleep(Duration::from_millis(110));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_cinq_etats_visuels_sont_couverts() {
        assert_eq!(
            TrayIconState::from(PlaybackState::Idle),
            TrayIconState::Ready
        );
        assert_eq!(
            TrayIconState::from(PlaybackState::Stopped),
            TrayIconState::Ready
        );
        assert_eq!(
            TrayIconState::from(PlaybackState::Capturing),
            TrayIconState::Generating
        );
        assert_eq!(
            TrayIconState::from(PlaybackState::Generating),
            TrayIconState::Generating
        );
        assert_eq!(
            TrayIconState::from(PlaybackState::Playing),
            TrayIconState::Playing
        );
        assert_eq!(
            TrayIconState::from(PlaybackState::Paused),
            TrayIconState::Paused
        );
        assert_eq!(
            TrayIconState::from(PlaybackState::Error),
            TrayIconState::Error
        );
    }

    #[test]
    fn chaque_frame_embarquee_est_un_png_non_vide() {
        let signe = [0x89, b'P', b'N', b'G'];
        for bytes in GEN_FRAMES
            .into_iter()
            .chain(PLAY_FRAMES)
            .chain([READY_PNG, PAUSE_PNG, ERROR_PNG])
        {
            assert!(bytes.len() > 100);
            assert_eq!(&bytes[..4], &signe);
        }
    }

    #[test]
    fn frame_bytes_boucle_sur_les_animations() {
        assert_eq!(
            frame_bytes(TrayIconState::Generating, 9).as_ptr(),
            frame_bytes(TrayIconState::Generating, 1).as_ptr()
        );
        assert_eq!(
            frame_bytes(TrayIconState::Playing, 4).as_ptr(),
            frame_bytes(TrayIconState::Playing, 1).as_ptr()
        );
    }
}
