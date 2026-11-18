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

## Limites actées

1. **Validation terrain requise avant clôture** : l'humain doit lancer Navi, sélectionner un texte dans un éditeur, presser « Lire la sélection », entendre le résultat, tester pause/reprise/arrêt. L'intégration compile et les tests unitaires sont verts, mais le device n'a pas été sollicité.
2. **premier clic sur Lire** : si le modèle n'est pas chargé, la commande déclenche le chargement en tâche de fond et renvoie « relancez dans quelques secondes » (UX assumée).
3. **Pause/Reprise ne vident pas la file** : conforme rodio (position conservée) ; `Arrêt` vide la file et invalide le consommateur.
4. **Au plus un segment orphelin** : si `Arrêt` survient entre la vérification du jeton et l'enqueue, un segment atterrit dans la file vidée — documenté (N-020).
5. **Voix par défaut** = `af_heart` (féminine EN) pour le mixte ; langue détectée pilote le phonémiseur espeak, pas la voix. Choix/forçage voix : N-053.
6. **`Stop` pendant `load_model`** : le chargement continue en tâche de fond et le modèle sera utilisable à la fin (le verrou est indépendant).
7. **`TtsConfig::default`** : utilisé partout (dev) ; `with_dir` pour les tests/envs custom.

## Traçabilité

- Carte : N-039. Sources : `src-tauri/src/pipeline.rs`, `audio/mod.rs`, `lib.rs`, `tts/mod.rs`, `src/App.tsx`, 3 nouveaux tests pipeline (suite **84** dont 15 intégration).
- Critères de la carte : Kokoro derrière `TtsEngine` ✓ ; chargement en tâche de fond ✓.
- Préfigurent N-040 (préchargement anticipé), N-041 (déclenchements rapides en conditions réelles).
