# S1-039 — Kokoro intégré derrière `TtsEngine` (carte N-039)

Date : 2026-11-18. Carte : N-039 (P1, L). Modules : `src-tauri/src/pipeline.rs` (nouveau), `src-tauri/src/audio/mod.rs` (refonte), `src-tauri/src/lib.rs` (câblage Tauri), `src-tauri/src/tts/mod.rs` (config), `src/App.tsx` (UI).

## Objectif de la carte

Brancher Kokoro derrière `TtsEngine` avec chargement en tâche de fond, et alimenter la file audio en segments synthétisés — avec annulation coopérative via le jeton de génération (N-020).

## Architecture livrée

```
App.tsx [Lire la sélection]
  └─ invoke start_reading → lib.rs
        1. state.begin_capture()            → UI badge « Capture… »
        2. capture_selection()               → Ctrl+C simulé + snapshot/restauration (N-043)
        3. clean_captured_text (N-035)       → texte propre ; erreur claire si vide
        4. detect_language (N-036)           → hint fr/en
        5. segment_text (N-037)              → Vec<Segment> (≤ 480 tokens chacun)
        6. AudioHandle (nouveau)             → thread audio dédié (OutputStream rodio/cpal)
        7. thread de synthèse                → synthesize_segments() — jeton vérifié entre segments
        8. SharedAudioSink logique           → AudioHandle::enqueue via canal mpsc (ordre garanti)

audio/thread → AudioPlayer (Sink rodio) → SamplesBuffer 24 kHz mono → périphérique par défaut
machine → événements `navi-state` → UI badge phase/progression/erreur
```

### Refonte audio (indispensable)

Le `OutputStream` de rodio contient des pointeurs bruts → **non Send/Sync** → impossible de le mettre dans le `State` Tauri. Solution : un thread audio dédié possède le `AudioPlayer` ; le state ne détient qu'un `AudioHandle` (canal mpsc, Send+Sync). Les commandes (`pause/resume/stop/reset/enqueue/set_volume`) arrivent par le canal — ordre garanti.

`AudioPlayer` refondu avec **génération de consommateur** (chaque `spawn_consumer` invalide le précédent) : `stop()` = invalidation + vidage file + sink coupé ; `reset()` = nouveau sink + nouveau consommateur (entrée de chaque nouvelle session — l'ancien consommateur meurt proprement).

### `TtsConfig`

`resolve_models_dir()` : `NAVI_MODELS_DIR` → `<repo>/models/kokoro/` (si ONNX présent) → `<repo>/models/` → `%APPDATA%/Navi/models`. Dev : `models/kokoro/` (copié manuellement depuis `scratch/s0_02/data/vocab.json`, **le modèle + voix + vocab sont requis sur chaque machine** — `models/` est gitignoré).

## État vs carte

| Item | Statut |
|---|---|
| Chargement en tâche de fond (`load_model`, idempotent) | ✓ |
| Boutons UI câblés (Lire / Pause|Reprendre / Arrêt / Précharger) | ✓ |
| Intégration pipeline capture→…→audio | ✓ (câblée, compilée, tests unitaires verts) |
| Tests pipeline avec stub (ordre, annulation, hint langue) | ✓ (3 tests) |
| Test audible de bout en bout | **non joué — recette humaine à faire** |
| Annulation coopérative entre segments | ✓ |
| Limites | voir ci-dessous |

## État final (2026-11-18, après recette terrain)

Validation humaine de la chaîne complète : **la voix lit le texte à voix haute** (« Domio est une application… »). Décisions et défauts rencontrés/corrigés pendant le développement :

| # | Défaut | Corrigé par |
|---|---|---|
| 1 | `voices-v1.0.bin` = archive `.npz` (zip de .npy), pas un dump binaire → slice mort, panic | parser `.npz` via `zip` + `npyz`, shape `[510,1,256]` validée |
| 2 | eSpeak NG non installé sur la machine | `models/phonemizer/espeak-ng.dll` (wheel) + `espeak-ng-data/` ; repli Program Files |
| 3 | `ort` en `load-dynamic` sans DLL à l'exécution | fonctionnalités par défaut (binaire embarquée) |
| 4 | Double clic « Charger le modèle » + lecture auto → deux `Environment` ort (« only one environment… ») | garde anti double-chargement (`AtomicBool`) |
| 5 | **`AudioHandle::drop` appelait `stop()` → fin de synthèse tuait la lecture** | `Drop` supprimé ; arrêt uniquement via « Arrêt » |
| 6 | Progression jamais mise à jour (`next_chunk` jamais appelé) | callback `on_segment_done` |
| 7 | Watcher Vite noyé par `scratch/` (page reload ×milliers) | `vite.config.ts` ignore `scratch/ target/ models/ dist/ worktrees/` |
| 8 | Préchargement manuel obligatoire | `load_model` appelé au montage de l'UI (idempotent) |

## Limites actées

1. La recette a été rejouée par l'humain : modèle chargé ✓, capture OK ✓, synthèse ✓, audio audible ✓, `Arrêt`/pause fonctionnels.
2. Premier clic sur « Lire la sélection » : ~4–6 s de latence (phonémisation + inférence) — normal pour ce matériel (RTF médian 0,36, mesuré S0).
3. Au plus un segment orphelin en vol (documenté N-020).
4. Voix par défaut = `af_heart` ; forçage FR via N-053 (« Choix voix/vitesse/volume »).
5. Le modèle ne persiste pas l'état de lecture à la fermeture ; la barre de progression repart à 0 à la prochaine session.

## Traçabilité

- Carte : N-039. Sources : `src-tauri/src/pipeline.rs`, `audio/mod.rs`, `lib.rs`, `tts/mod.rs`, `src/App.tsx`, `vite.config.ts`, tests 84 verts + `kokoro_smoke` (5 s, peak 0,62).
- Critères de la carte : Kokoro derrière `TtsEngine` ✓ ; chargement en tâche de fond ✓.
- Préfigurent N-040 (préchargement segments), N-041 (intégration machine d'états ↔ audio robuste).

## Traçabilité

- Carte : N-039. Sources : `src-tauri/src/pipeline.rs`, `audio/mod.rs`, `lib.rs`, `tts/mod.rs`, `src/App.tsx`, 3 nouveaux tests pipeline (suite **84** dont 15 intégration).
- Critères de la carte : Kokoro derrière `TtsEngine` ✓ ; chargement en tâche de fond ✓.
- Préfigurent N-040 (préchargement anticipé), N-041 (déclenchements rapides en conditions réelles).
