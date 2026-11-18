# S1-032 — Machine d'états de lecture, phases complètes et événements UI (carte N-032)

Date : 2026-11-18. Carte : N-032 (P1, M). Modules : `src-tauri/src/state/mod.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`.
But de la carte : **inactif / capture / génération / lecture / pause / erreur, avec événements vers l'UI**.

## Contrat d'états

```
                begin_capture()
   Idle / Stopped / Error ──────────────────► Capturing
                                                   │ begin_generation_phase()
                                                   ▼
                                              Generating ──play()──► Playing
                                                   │                     │ pause()
                                                   │                     ▼
                                                   │                   Paused ──resume()──► Playing
                                                   ▼                     │
                                              Stopped ◄───── stop() ─────┘ (depuis tous les états sauf Idle)
                                                   ▲
                                                   └────────── set_error() ──► Error
```

Transitions d'écriture : `Idle|Stopped|Error → Capturing → Generating → Playing ⇄ Paused → (stop) Stopped`.
`Error` est terminal (relance = `begin_capture` → état machine réinitialisé côté capture).

## Contrat d'événements (`navi-state` côté Tauri, tagged serde)

```json
{ "type": "phase-changed", "from": "idle",  "to": "capturing" }
{ "type": "progress",       "current": 2,   "total": 5 }
{ "type": "error",          "message": "…" }
```

Événements stables pour l'UI : `phase-changed`, `progress`, `error`.
Les phases sont émises en minuscule (`idle|capturing|generating|playing|paused|stopped|error`).

## Implémentation

- `PlaybackStateMachine` : registre d'écouteurs (`subscribe`, appels **synchrones**, écouteurs non bloquants exigés — pour l'UI : seuls des `emit` Tauri).
- `set_phase` : mise à jour + émission atomiques (à l'échelle de l'appelant — tous les appels sont dans le thread appelant).
- `next_chunk` émet `Progress` ; `stop` remet à zéro et émet `Progress(0, total)` ; `set_error` émet `Error` puis `PhaseChanged`.
- `get_status` (commande Tauri) : snapshot `{ phase, current_chunk, total_chunks }` pour le rendu initial.
- `lib.rs::run` : crée la machine, la `manage()`, branche un écouteur `navi-state` (Tauri `emit`).
- UI React : badge de phase coloré, jauge `progress`, écoute `navi-state`, boutons « Lire/Pause/Arrêt » **désactivés** (branchés par N-039/N-041/N-045) — honnêteté produit : pas de faux boutons.

## Limites / à acter

1. **La machine ne lance rien** — elle ne capture pas, ne synthétise pas, ne joue pas. La chaîne réelle arrive avec N-039 (Kokoro) + N-041 (intégration) + N-045 (raccourcis). Cette carte pose uniquement le contrat d'états + événements.
2. **Écouteurs synchrones** : un auditeur bloquant casse la machine. L'UI Tauri enregistre des écouteurs qui ne font qu'`emit` (non bloquant). Si un jour un écoute coûte cher → pompe découplée (non nécessaire pour l'instant, signalée comme règle dans le module).
3. **`error` n'automatiquement aucune reprise** : l'UI doit proposer de relancer (bouton Rejouer) — la logique de reprise métier arrive avec N-041.
4. **L'état `Generating` est entrée via `begin_generation_phase()`** — l'appelant doit avoir fait un `begin_capture` avant. Ce n'est pas un état « par défaut » : la génération Kokoro réelle arrive avec N-039.
5. **Sérialisation stable testée** (`les_evenements_se_serialisent_en_json_stable`) : si le contrat JSON change, l'UI casse silencieusement — la rupture de compatibilité exige une décision (N-052/N-051 à venir).

## Vérification

```
cargo test  → 37 passed, 0 failed (dont 5 pour la carte : parcours complet capture→arrêt,
               capture refusée hors repos, écouteurs reçoivent les transitions dans l'ordre,
               Error active phase-changed, JSON stable)
npx tsc --noEmit / npm run build → OK
```

## Traçabilité

- Carte : N-032. Sources : `src/state/mod.rs`, `src/lib.rs`, `src/App.tsx`, `App.css`, `cargo test`.
- Critère de la carte : « implémenter la machine d'états de lecture avec événements vers l'UI » → contrat complet, événements branchés, UI minimale honnête.
