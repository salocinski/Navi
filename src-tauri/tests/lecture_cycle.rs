//! Cycles complets de lecture sur machine d'états + file audio (carte N-041).
//!
//! Exerce la chaîne start → pause → resume → stop → restart sans device ni Kokoro.
//! La vérification audible reste côté humain (Tais, Navi, fenêtre).

use navi_lib::state::{PlaybackState, PlaybackStateMachine};
use navi_lib::audio::SegmentQueue;

fn simulate_cycle(machine: &PlaybackStateMachine, segments: usize) {
    machine.set_text("texte de test".to_string(), segments);
    machine.begin_capture().unwrap();
    machine.begin_generation_phase().unwrap();
    machine.play().unwrap();
    for _ in 0..segments {
        assert!(machine.next_chunk().is_some());
    }
}

#[test]
fn cycle_complet_lecture_pause_reprise_arret() {
    let machine = PlaybackStateMachine::new();

    // Cycle 1 : lecture, pause, reprise, arrêt
    simulate_cycle(&machine, 3);
    machine.pause().unwrap();
    assert_eq!(machine.state(), PlaybackState::Paused);
    machine.resume().unwrap();
    assert_eq!(machine.state(), PlaybackState::Playing);
    machine.stop();
    assert_eq!(machine.state(), PlaybackState::Stopped);
    assert_eq!(machine.progress(), (0, 3));

    // Cycle 2 : relecture possible depuis Stopped
    machine.play().unwrap(); // prêt à rejouer ? oui, depuis Stopped
    assert_eq!(machine.state(), PlaybackState::Playing);
    machine.stop();

    // Cycle 3 : enchaînement capture-génération-lecture sans passer par Stopped
    machine.begin_capture().unwrap();
    machine.begin_generation_phase().unwrap();
    machine.set_text("autre".to_string(), 2);
    machine.play().unwrap();
    machine.stop();
    assert_eq!(machine.progress(), (0, 2));
}

#[test]
fn file_audio_cycle_multi_sessions() {
    // Un même player en enchaîne plusieurs : stop vide la file, reset repart propre
    let mut q = SegmentQueue::new();
    q.push(vec![0.1]);
    q.push(vec![0.2]);
    q.push(vec![0.3]);
    assert_eq!(q.pop(), Some(vec![0.1]));
    q.clear();
    assert_eq!(q.len(), 0);
    q.push(vec![0.5]);
    assert_eq!(q.pop(), Some(vec![0.5]));
}

#[test]
fn transitions_interdites_refusees_partout() {
    let machine = PlaybackStateMachine::new();
    // Pause depuis Idle : refusée
    assert!(machine.pause().is_err());
    // Stop depuis Idle : autorisé (no-op) et laisse Stopped
    machine.stop();
    assert_eq!(machine.state(), PlaybackState::Stopped);
    // Re-binaire : reprise sans pause refusée
    assert!(machine.resume().is_err());
    // Dans l'état Error : rien n'accepte de rejouer sans capture d'abord
    machine.set_error("modèle absent".to_string());
    assert!(machine.play().is_err());
    // Captation possible depuis Error (l'humain peut retenter) — set_text reset progression
    machine.begin_capture().unwrap();
    assert_eq!(machine.state(), PlaybackState::Capturing);
}
