# S1-050 — Anneaux d'état de l'icône (carte N-050)

Date : 2026-11-19. Carte : N-050 (P1, M). Branche : `feat/TW-050-anneaux-etat`.
DA : maquette C (`docs/pistes-interface.html`) — panneau A + anneaux C, sans pastille B (décision TW-068/069).
Sources : `src-tauri/icons/icon.png` (sphère néon ronde, fournie par l'humain via `navi icon.jfif`), `src-tauri/src/state/mod.rs` (machine d'états), `src-tauri/src/ui/mod.rs` (tray).

## Ce que fait la carte

L'icône de la zone de notification reflète la phase de lecture en permanence — plus besoin d'ouvrir quoi que ce soit pour savoir où en est Navi.

| Phase machine | Icône | Constante |
|---|---|---|
| Idle / Stopped | aucun anneau (sphère nue) | `ready.png` |
| Capturing / Generating | anneau bleu qui tourne (8 frames, ~0,9 s/tour) | `gen_0..gen_7.png` |
| Playing | anneau cyan qui pulse (3 frames, ~660 ms/cycle) | `play_0..play_2.png` |
| Paused | anneau ambre fixe | `pause.png` |
| Error | pastille rouge avec « ! » (bas droite) | `error.png` |

Les frames sont des PNG 32×32 embarquées (`include_bytes!`). Un fil de fond (`ui::tray_icons::spawn_icon_animator`) lit `machine.state()` toutes les ~110 ms et **ne pousse l'icône que lorsque (état, frame) change** — pas de set_icon inutile, pas de scintillement. Aucune logique UI dans la machine d'états (contrat N-032 intact).

Mapping des phases : `Capturing` et `Generating` → « Génération » (la capture est transitoire) ; `Stopped` → « Prêt » (icône au repos, cohérent avec l'arrêt) ; erreur = terminale tant qu'aucune action ne relance la machine.

## Génération des frames

`node tools/gen_tray_icons.js` (outil ponctuel, **hors build** ; prérequis : `npm install --no-save sharp` dans une session jetable — sharp n'est pas une dépendance du projet).

Le générateur compose la sphère (26px, coins transparents) + piste blanche faible (0.22) + arc/cercle coloré avec halo (double trait), sur canvas 32px. Aperçus 16/20/24 px déposés dans `scratch/tray-preview/`.

## Lisibilité à 16, 20 et 24 px (revue visuelle d'images, ×6 nearest)

| Icône | 16 px | 20 px | 24 px |
|---|---|---|---|
| ready | ✅ orbe identifiable | ✅ | ✅ |
| gen (arc bleu) | ✅ arc + piste visibles | ✅ | ✅ |
| play (cyan) | ✅ anneau plein net | ✅ | ✅ |
| pause (ambre) | ✅ net | ✅ | ✅ |
| error (pastille) | ✅ « ! » lisible | ✅ | ✅ |

Aperçus produits par `tools/gen_tray_icons.js` ; vérification sur vraie barre Windows 11 (tray, DPI 100/125/150 %, thèmes clair/sombre) = **recette humaine requise** (aucune mesure automatique possible sur le tray dans cette session).

## `prefers-reduced-motion`

Équivalent Windows : `SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION)` (accessibilité > effets visuels). Si les animations client sont coupées :

- génération → arc bleu **fixe** (frame 0, état indiqué par la couleur, zéro mouvement) ;
- lecture → anneau cyan **fixe** (frame 1, opacité moyenne) ;
- pause/erreur/prêt : déjà statiques.

La requête est faite une fois au démarrage du fil (pas de surveillance live) — conforme au caractère « réglage système » du paramètre.

## Vérification

```text
cargo fmt / clippy --all-targets -- -D warnings   → OK
cargo test                                        → 73 + 3 + 15 verts (3 nouveaux tests tray_icons)
```

Recette humaine restante (tracée, non couverte par CI) : lancer Navi, déclencher chaque phase (lire / pause / arrêt / erreur via une app sans sélection) et confirmer le rendu sur le tray en clair et sombre.

## Traçabilité

Carte N-050. Sources : `ui/mod.rs` (tray, id `navi-tray`), `ui/tray_icons.rs`, `tools/gen_tray_icons.js`, `docs/pistes-interface.html` (DA), `docs/s1-69` (icônes multi-tailles, sphère néon).
