# S1-077 — Pastille flottante du cercle animé (carte N-077)

Date : 2026-11-19. Carte : N-077 (P2, M). Branche : `feat/TW-077-pastille-flotte`.
**Révision explicite de la décision TW-68** (voir `docs/decisions.md` — « Pastille flottante du cercle animé (TW-077, amendement TW-68) »). Demande humaine, 2026-11-19.

## Ce que fait la carte

Une pastille ronde 72×72 px, toujours au-dessus de tout, sans focus, non cliquable, visible **de la combinaison (Capture) jusqu'à la fin de l'audio**, pour rendre le statut lisible sans ouvrir l'app. Vivante de `Capturing` → `Generating` → `Playing` / `Paused` ; rouge en erreur ; masquée sur `Stopped`/`Idle`.

| Phase machine | Pastille |
|---|---|
| Capturing / Generating | arc bleu en rotation (CSS) |
| Playing | anneau cyan qui pulse (CSS) |
| Paused | anneau ambre fixe |
| Error | pastille rouge avec « ! » |
| Stopped / Idle | masquée |

## Implémentation

- `public/floating.html` — page unique, SVG inline + CSS inline, **zéro asset, zéro dépendance**. Suit la machine via `window.__TAURI__.event.listen('navi-state', …)` (mêmes événements que la fenêtre principale). `prefers-reduced-motion` : media query native → animations coupées (arc/anneau fixes).
- `src-tauri/src/ui/flotteur.rs` (`ui::flotteur`) — création programmatique de la fenêtre `flotteur` : `decorations(false)`, `transparent(true)`, `shadow(false)`, `always_on_top(true)`, `skip_taskbar(true)`, `focused(false)`, `ignore_cursor_events(true)` (non cliquable, ne vole jamais le clic à l'app capturée), `visible(false)` ; position bas-droite de la zone de travail du moniteur principal (`work_area`), marge 16 px.
- `src-tauri/src/lib.rs` (setup, après le tray) — abonnement à la machine : `PhaseChanged` → `show()` sur Capturing/Generating/Playing/Paused/Error, `hide()` sinon. Le reste de la logique reste dans la machine (aucun état UI ajouté dans `state/`).
- `withGlobalTauri: true` dans `tauri.conf.json` pour l'API injectée `__TAURI__` (la page n'utilise pas le bundle npm).

## Choix et écarts assumés

- **Pas de clic sur la pastille** : pure information ; le quitter/arrêter restent le tray, le panneau et les raccourcis. La pastille se ferme toute seule en fin de session.
- **Erreur = reste visible** (rouge) jusqu'à la prochaine action (elle ne disparaît pas sous le nez de l'utilisateur) ; `Stopped` ferme tout.
- **Non interactif** : `ignore_cursor_events` — zéro frontière cliquable gênante sur les apps plein écran.
- 72 px est une suggestion de départ ; DPI et position (bas-droite) sont du domaine du goût — à ajuster à la recette humaine.

## Vérification

```text
cargo fmt / clippy --all-targets -- -D warnings   → OK
cargo test                                         → 70 + 3 + 15 verts
npm run build (tsc + vite)                         → OK, dist/floating.html émis
prettier --check                                   → OK
```

**Recette humaine requise** (aucune automation possible) : valider le déclenchement par le raccourci, l'animation, la non-prise de focus (l'app capturée garde le clavier), la non-interception des clics, la disparition à la fin de l'audio, et le rendu en clair/sombre + DPI 100/125/150 %.

## Traçabilité

Carte N-077. Sources : `docs/decisions.md` (amendement TW-68), `docs/pistes-interface.html` (variante B historique), `state/mod.rs` (`StateEvent`), `ui/mod.rs`, `ui/flotteur.rs`, `public/floating.html`.
