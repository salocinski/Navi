# S1-040 — Préchargement des segments et réglage de leur taille (carte TW-040)

Date : 2026-11-19. Carte : N-040 (P1, M). Sources : benchmarks S0-07b/c, `docs/s1-37-segmentation.md`, `src-tauri/src/pipeline.rs`.

## Ce que disent les benchmarks (S0-07b, machine de référence)

| Cellule | tokens | TTFA | RTF médiane |
|---|---|---|---|
| fr_court_ff_siwis | 80 | 1 980 ms | 0,36 |
| fr_moyen_ff_siwis | 271 | 6 111 ms | 0,37 |
| fr_long_ff_siwis | 820 (2 seg) | 8 835 ms | 0,40 |
| en_moyen_af_heart | 285 | 6 179 ms | 0,35 |
| en_long_af_heart | 830 (2 seg) | 9 612 ms | 0,36 |

Observations : TTFA croît ~linéairement avec le nombre de tokens ; au-delà de ~270 tokens, la latence perçue dépasse 6 s ; au-delà de ~480, on subit des césures (plus de chunks). Le modèle n'accepte que ~510 tokens par inférence. Le mieux en WER/qualité est de **ne pas dégrader** l'expression — garder des segments longs avec césures aux frontières de phrase (déjà le cas, N-037).

## Décision de réglage

- **`max_tokens_per_chunk()` = 480** (défaut, même contrat qu'en S0) — conserve la marge sous 510, évite les césure trop fréquentes, aucune preuve terrain que 200/300/400 soit meilleure en qualité ; le coût est une latence un peu plus élevée sur texte moyen — déjà compensée par le préchargement du modèle (N-039, pièce préchargée au démarrage).
- **Surcharge via `NAVI_MAX_SEGMENT_TOKENS`** (16–510, sinon repli à 480) : force l'écoute à comparer 200/320/480 sur la même phrase quand l'humain veut trancher à l'oreille — plus besoin de recompiler pour la preuve qualité.
- RTF reste le critère temps réel (0,36 médiane) ; le réglage de segment n'en dévie pas.

## Ce qui est préchargé aujourd'hui (honnêteté technique)

| Étape | Statut |
|---|---|
| Modèle Kokoro préchargé au démarrage de l'UI | ✓ (N-039, `App.tsx` appelle `load_model`) |
| Premier segment synthétisé dès le clic « Lire », lu dès qu'il est prêt — playback interleaved | ✓ (N-039 : pointeur de production, file FIFO AudioPlayer) |
| Segments suivants synthétisés pendant la lecture du premier — anticipation automatique de la file entière en arrière-plan | ✓ (synthèse complète en tâche de fond avant la consommation) |
| Cold start mesuré après préchargement | non mesuré en terrain (à mesurer si besoin) |
| Segments préchargés **avant** le clic utilisateur (anticipation de la sélection) | **NON** — exige prédicter la sélection, hors scope (gain cédulé par produit, probablement impossible sans UI de survol) |

## Pas fait ici

- Re-taille audio directe en fonction de la durée réelle (Kokoro ne donne pas la durée avant inférence — estimée par tokens).
- Trimming des silences de tête (benchmarks non trimés ; non fait — à faire dans le pipeline si la qualité le demande).

## Vérification

```
cargo test → 70 passed, 0 failed (dont max_tokens_env_override)
```

## Traçabilité

- Carte : N-040. Sources : `src-tauri/src/text/segmentation.rs`, `docs/s0-07c-cpu-verdict.md`, `docs/s0-07b-measurement.md`, `docs/s1-37-segmentation.md`.
- Critère de la carte : taille ajustée sur benchmarks ✓ (480 conservés avec justification) ; mécanisme de préchargement démontré et documenté ✓.
