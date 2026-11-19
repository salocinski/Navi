# S1-041 — Lecture, pause, reprise, arrêt intégrés à la machine d'états (carte N-041)

Date : 2026-11-19. Carte : N-041 (P1, M). Modules : `src-tauri/src/state/mod.rs`, `src-tauri/src/pipeline.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/audio/mod.rs`, `src/App.tsx`.

## Critère de la carte

« Pas de désynchronisation majeure ». Le mécanisme a été validé en conditions réelles par l'humain pendant TW-039 (la voix lit Domio, pause/reprise/arrêt audibles — les clics liaient bien l'UI à la chaîne). Ce que ce ticket apporte : la **vérification formelle des transitions** sans recourir au périphérique, et la prévention des doubles démarrages côté UI.

## Ce qui a été fait

### 1. Tests d'intégration crate-level (`tests/lecture_cycle.rs`)
Sans périphérique ni Kokoro — cycle complet dans la machine et la file :

| Test | Ce qu'il prouve |
|---|---|
| `cycle_complet_lecture_pause_reprise_arret` | Idle → Capturing → Generating → Playing → Paused → Playing → Stopped ; arrêt remet la progression à 0 ; relecture possible depuis Stopped ; enchaînement immédiat capture-génération-lecture sans passer par Stopped |
| `file_audio_cycle_multi_sessions` | La `SegmentQueue` enchaîne plusieurs sessions : fifo respectée, `clear()` vide, reprise propre |
| `transitions_interdites_refusees_partout` | Pause depuis Idle refusée ; Stop depuis Idle no-op ; Reprise sans pause refusée ; Lecture depuis `Error` refusée ; Captation possible depuis `Error` |

Suite complète : **73 passed, 0 failed**.

### 2. Frontend (`src/App.tsx`)
Le bouton **« Lire la sélection »** est désactivé pendant `capturing | generating | playing` (les états où un redémarrage serait remontant sur le travail en cours). Précédemment, un second clic clôturait dans `Error` via le refus de la machine — comportement défensif mais sans message contextuel. L'UI évite désormais le double-clic par construction ; si l'utilisateur arrive dans `Error`, la remontée reste possible via le bouton (begin_capture accepte Error).

### 3. Ce qui n'a PAS été fait (volontairement)
- Pas de nouveau test audible côté CI (device requis — reste dans la recette humaine).
- Pas de « fin de lecture » automatique : la machine reste en `Playing` une fois les échantillons joués ; seul `Arrêt` clôture. À traiter si besoin dans N-041-suivi ou avec TW-041 (pipeline) : la `SegmentQueue` pourrait exposer un événement « vide + génération terminée » — hors scope ici.
- Pas de raccourcis clavier/hotkeys branchés (N-045).

## Traçabilité

- Carte : N-041. Sources : `src-tauri/tests/lecture_cycle.rs`, `src/App.tsx`, `cargo test` (73 passed), `npm run build`.
- Dépendances : N-032 (machine d'états), N-020 (FIFO/stop/jeton), N-039 (Kokoro intégré + reset).
- Critère de la carte : « pas de désynchronisation majeure » → transitions refusent les états interdits, progression/recapture remise à zéro, double démarrage bloqué côté UI.
