# S1-018 — Raccourcis par défaut et leurs conflits (carte TW-018)

Date : 2026-11-18. Carte : N-018 (P2, S). Module : `src-tauri/src/hotkey/mod.rs`.
Question : quel jeu de raccourcis par défaut retenir, et que se passe-t-il en cas de conflit ?

## État de la discordance (S0-17 / critère 5)

- Backlog initial (BACKLOG.md EP-04) : **Ctrl+Alt+Espace / P / S**.
- Prototypes Sprint 0 (`global-hotkey`, `scratch/hotkey_017`) : **Ctrl+Shift+R/P/S**.
- À trancher ici, avec des faits.

## Mesure du 2026-11-18 (machine de référence, `cargo run --example hotkey_check`)

| Combinaison | Résultat |
|---|---|
| Ctrl+Maj+R (lire, candidat) | ✅ OK (libre mais non retenu pour « lire » après amendement du 2026-11-18) |
| Ctrl+Maj+Espace (lire) | ✅ OK (amendement du 2026-11-18 : défaut « lire » demandé) |
| Ctrl+Maj+P (pause/reprise) | ✅ OK |
| Ctrl+Maj+S (arrêt) | ✅ OK |
| Ctrl+Alt+Espace (lire) | ❌ **HotKey already registered** — réservée par l'OS ou une application (non identifiable depuis l'extérieur ; Windows ne l'attribue qu'à un seul détenteur) |
| Ctrl+Alt+P (pause) | ✅ OK |
| Ctrl+Alt+S (arrêt) | ✅ OK |

Fait nouveau : le candidat initial (Ctrl+Alt+Espace) est **déjà occupé** sur la machine de référence ; Ctrl+Maj+R restait libre mais cède sa place à Espace pour « lire » (demande du 2026-11-18).
Les conflits peuvent différer selon les machines — la détection se fait **à l'enregistrement**, jamais en amont.

## Décision

**Par défaut = Ctrl+Maj+Espace (lire) / Ctrl+Maj+P (pause-reprise) / Ctrl+Maj+S (arrêt)** (amendement du 2026-11-18 sur demande : Espace pour « lire », plus ergonomique au clavier ; Ctrl+Maj+R reste un candidat possible via N-052).
**Ctrl+Alt+Espace est écarté** (conflit constaté sur la machine de référence).
Consigné dans `docs/decisions.md` (TW-018).

## Comportement en cas de conflit (critère de la carte)

`register_defaults` est tolérant aux pannes partielles :

1. Chaque raccourci est tenté indépendamment — un échec n'empêche pas les autres.
2. Chaque échec est **détecté** (erreur de `RegisterHotKey`) et **isolé** avec un message clair :
   `RegistrationReport { registered: [...], failed: [(action, combo, erreur)] }`.
3. `RegistrationReport::failure_message()` produit le texte affichable :
   « Raccourci(s) indisponible(s) — déjà réservés par l'OS ou une autre application : Ctrl+Alt+Espace → Arrêt : HotKey already registered » (exemple).
4. L'UI (N-045/N-052) proposera la re-configuration des raccourcis ; la surface d'erreur existe dès maintenant et sera remontée au tray/panneau à la fusion des cartes correspondantes.

Limites :
- Windows ne dit **pas quelle** application détient la combinaison ; seule la détection de l'échec est disponible.
- Un conflit postérieur à l'enregistrement est impossible : la combinaison est tenue par Navi jusqu'à fermeture.
- Les tests unitaires couvrent `combo_label`, `failure_message`, la complétude/curation des rapports (4 tests, suite 25 passed).
- La validation « conflits entre différents PCs » reste de la responsabilité de chaque utilisateur — d'où les raccourcis configurables (N-052).

## Traçabilité

- Carte : N-018. Sources : `src-tauri/src/hotkey/mod.rs` (module + `examples/hotkey_check.rs`), mesures ci-dessus.
- Critère de la carte : « Comportement en cas de conflit documenté » ✓ (section ci-dessus).
- Le signalement utilisateur final (toast/tray/panneau) est rattaché à N-045/N-052, pas à cette carte.
