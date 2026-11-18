# S1-020 — Pause, reprise, arrêt et file audio (carte TW-020)

Date : 2026-11-18. Carte : N-020 (P1, M). Dépendances : rodio 0.19, modules `audio` et `state`.

## Constats avant correction

| Défaut | Preuve | Conséquence |
|---|---|---|
| File `pending` en LIFO (`Vec::pop`, lecture à l'envers) | `src-tauri/src/audio/mod.rs` (avant correctif) | Chunks joués dans l'ordre inverse de la génération |
| `stop()` ne vidait pas `pending` | idem | « Arrêt : vider la file » de la carte non satisfait ; segments orphelins jouables après arrêt |
| `stop()` consommait le sink (`Option::take`) sans recréation | idem | Aucun son possible après un arrêt ; `enqueue`/`pause` no-op silencieux |
| Aucun mécanisme d'annulation de génération | `state` sans jeton | Un worker TTS pouvait écrire dans la file après un arrêt |
| `Default for AudioPlayer` forçait `expect` (périphérique audio requis) | idem | Panique en contexte headless / tests |
| `Default` supprimé ; pause hors lecture non testée ; état `Error` non testé | — | Comportements incohérents non détectables |

## Correctifs

### `audio`
- `SegmentQueue` : file **FIFO** dédiée (`push_back`/`pop_front`), logique pure testable sans périphérique audio (`file_fifo_ordre_respecte`, `file_clear_vide_la_file`).
- `enqueue` inchangé pour l'appelant ; `queue_len()` ajouté ; `clear_queue()` public (arrêt = vider la file).
- `stop()` : `stopped=true`, `paused=false`, **vidage effectif de la file**, `notify_all`, arrêt du sink (position abandonnée volontairement — un arrêt n'est pas une pause).
- `reset()` ajouté : nouveau sink + file vidée + drapeaux réinitialisés → une nouvelle session de lecture est possible après arrêt (repart de la position 0, cf. `stop` qui remet `current_chunk` à zéro).
- `start_consumer` : sort si `stopped` entre l'attente condvar et la lecture ; n'ajoute pas de chunk si l'arrêt est survenu ; sortie propre de la boucle.
- `is_playing`, `set_volume` : inchangés (comportement rodio, position conservée par `sink.pause()`).

### `state`
- Jetons de génération : `begin_generation()` (incrémente, réarme l'annulation), `request_cancel_generation()`, `is_generation_cancelled(id)`. Un `stop()` invalide implicitement tout jeton antérieur.
- Un worker TTS doit vérifier `is_generation_cancelled(id)` **entre chaque segment** et abandonner sans `enqueue`.
- `stop()` remet `current_chunk` à zéro (position perdue volontairement).
- Tests : pause hors lecture refusée, reprise sans pause refusée, position remise à zéro, arrêt ↔ annulation de génération, invalidation du jeton précédent, `Error` terminal sans reset explicite.

## Vérification

```text
cargo test --manifest-path src-tauri/Cargo.toml
test result: ok. 13 passed; 0 failed (audio 2, state 8, capture 1, config 1, utils 1)
```

## Limites / à acter

1. **Sans perte de position** : garantie au niveau (a) de l'indice de segment (`next_chunk`, `stop` remet à zéro ; `pause` n'y touche pas) et (b) de la position dans le sink rodio (`sink.pause()` conserve la position ; `sink.stop()` abandonne la position — arrêt ≠ pause). **Aucune désynchronisation majeure** : l'ordre FIFO est désormais garanti et la file est vidée à l'arrêt.
2. Vérification **audible** de bout en bout (avec des WAV Kokoro réels sur ≥ 2 chunks) non effectuée ici : nécessite la boucle TTS branchée (cartes N-039/N-041). À re-jouer lors de l'intégration verticale.
3. L'annulation de génération est **coopérative** : elle repose sur le worker vérifiant le jeton entre segments ; elle n'interrompt pas une inférence ONNX en cours (pas d'API d'abandon propre côté `ort`). Un stop pendant une génération peut produire au plus un segment orphelin en mémoire — jamais joué car la file est vidée et le jeton invalide.
4. `Default for AudioPlayer` supprimé : la création exige un périphérique ; les tests n'en dépendent pas.

## Traçabilité

- Carte : N-020. Sources : `src-tauri/src/audio/mod.rs`, `src-tauri/src/state/mod.rs`, `cargo test` (ci-dessus).
- Critères de la carte : arrêt vide la file (`SegmentQueue::clear` via `stop`) ✓ ; annulation propre de la génération (jeton) ✓ ; sans perte de position hors arrêt (pause = position conservée) ✓ ; démonstration multi-chunks réelle → N-041.
