# S1-021 — Déclenchements rapides et répétés (carte TW-021)

Date : 2026-11-18. Carte : N-021 (P2, S). Modules : `src-tauri/src/state/mod.rs`, `src-tauri/src/audio/mod.rs`.
Questions de la carte : relance pendant lecture, stop pendant génération.

## Tests ajoutés (suite 32 passed, 0 failed)

### Machine d'états (`state`)

| Test | Scénario | Invariant vérifiée |
|---|---|---|
| `relance_pendant_lecture_est_refusee_mais_stable` | `play` pendant `Playing` | erreur retournée, état reste `Playing`, position intacte |
| `pause_reprise_alternees_100_fois` | pause/reprise ×100 | l'état final est `Playing`, aucune dérive |
| `stop_pendant_generation_annule_le_jeton` | stop avec génération en cours | jeton invalidé ; relance avec nouveau jeton propre |
| `le_stop_rapide_en_boucle_reste_coherent` | `begin_generation` → play → chunk → stop ×100 | invariant complet : `Stopped`, position à zéro, jeton invalidé à chaque itération |
| `pause_apres_stop_est_refusee` | pause après stop → refus ; play → `Playing` | pas de double transition |

### File audio (`SegmentQueue`, `audio`)

| Test | Scénario | Invariant |
|---|---|---|
| `file_stressee_1000_segments_fifo` | 1000 push puis 1000 pop | ordre FIFO strict, file vide |
| `file_arret_pendant_debit_en_cours` | 2 consommations puis `clear()` (arrêt) | restants vidés, aucun segment orphelin lisible |

## Points subtils assumés (hors test unitaire, à valoir en recette)

1. **Stop pendant l'inférence ONNX en vol** : le jeton est coopératif — un worker doit vérifier `is_generation_cancelled` entre segments (N-039/N-041) ; l'inférence en cours termine, son résultat sera ignoré (pas d'API d'abandon dans `ort` pour l'instant).
2. **Audio réel / Sink rodio** : les tests ci-dessus ne touchent aucun périphérique (déterminisme, CI). Le comportement audible des relances rapides se valide uniquement sur la machine (protocole manuel ci-dessous).
3. **Interruptions raccourci × raccourci** : la détection de pressions multiples très rapprochées au niveau Win32 est hors test ; chaque action passe par `HotkeyManager` → machine d'états ; les transitions invalides sont refusées (testées).

## Protocole de recette manuelle (machine de référence)

1. Lancer la boucle de lecture (quand Navi lira effectivement — N-039/N-041) puis presser **Ctrl+Maj+S** pendant la génération d'un segment long : aucun segment ne doit être enchaîné après l'arrêt ; la barre/icône passe à l'état arrêté.
2. Presser **Ctrl+Maj+Espace** puis **Ctrl+Maj+P** aussitôt ×10 : aucune divergence d'icône ni de statut.
3. Presser **Ctrl+Maj+S** pendant la génération puis **Ctrl+Maj+Espace** immédiatement : la lecture repart depuis le début, pas du milieu.

## Traçabilité

- Carte : N-021. Sources : modules `state`/`audio`, `cargo test` (32 passed).
- Dépendances : N-020 (file FIFO, stop, jeton) — les tests héritent de ces garanties.
- Critère : scénarios « relance pendant lecture, stop pendant génération » couverts par les tests ci-dessus et le protocole manuel.
