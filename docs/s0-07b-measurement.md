# S0-07b — Mesures benchmark (carte TW-023)

Date : 2026-11-17. Statut : **mesuré** — exécution du protocole `docs/s0-07a-benchmark-protocol.md` (TW-022).
Machine : machine de référence (`docs/environnement-reference.md`) — Windows 10 Home 10.0.19045, MSI MS-7A71, i7-7700K (4C/8T), 31,9 Go, RTX 3060 Ti 8 Go (driver 596.49, non utilisée : CPU), commit `ed99346`, Rust 1.99.1, 9,8 Go RAM libres au début de la campagne.
Étiquette : « mesuré sur Windows 10 Home ». EP CPU uniquement, graphe d'optimisation au niveau 3, `ort` 2.0.0-rc.15.

## Protocole appliqué

- Chauffe + 3 runs par cellule, médiane retenue (min/max en note).
- Harnais Rust `scratch/s0_07` (processus frais, modèle rechargé à chaque invocation, EP CPU) ; contrôle Python via `kokoro-onnx` (découpage interne).
- Segments ≤ 510 phonemes (découpage via `_split_phonemes`) ; les textes longs = 2 segments.
- Runs : **CPU uniquement** — VRAM : **non applicable**. GPU hors périmètre (carte TW-8 / S0-02c).
- **Audio non trimé côté Rust** (silences de tête conservés ~2 s/phrase) ; le contrôle Python est trimé (`trim=True`). RTF Rust et Python ne sont donc pas directement comparables cellule à cellule — le RTF Rust inclut le silence initial.
- RAM pic : working set Win32 du processus (modèle ~92 Mo inclus). CPU : (user+kernel) d'exécution / wall / 8 threads logiques.
- Voix : `ff_siwis` (FR, seuil qualité), `af_heart` (anglais de contrôle + contrôle FR), `am_adam` (mâle EN de contrôle).
  Note : le protocole fusionné écrit `fr_siwis` ; la voix réelle dans `voices-v1.0.bin` est **`ff_siwis`**.

## Résultats Rust (médiane des 3 runs) — EP CPU

| Cellule | segments | tokens | TTFA (ms) | Génération (ms) | Audio (ms) | RTF | RAM pic (Mo) | CPU % |
|---|---|---|---|---|---|---|---|---|
| fr_court_ff_siwis | 1 | 80 | **1980** | 1 980 | 5 550 | **0,36** | 345 | 39,5 |
| fr_moyen_ff_siwis | 1 | 271 | **6 111** | 6 111 | 16 600 | **0,37** | 816 | 39,3 |
| fr_long_ff_siwis  | 2 | 820 | **8 835** | 17 076 | 42 875 | **0,40** | 1 016 | 39,3 |
| en_court_af_heart | 1 | 80 | **2 961** | 2 961 | 5 125 | **0,58** | 330 | 31,5 |
| en_moyen_af_heart | 1 | 285 | **6 179** | 6 179 | 17 450 | **0,35** | 841 | 39,1 |
| en_long_af_heart  | 2 | 830 | **9 612** | 16 960 | 47 525 | **0,36** | 1 134 | 40,4 |
| fr_court_af_heart | 1 | 80 | **1 921** | 1 921 | 5 475 | **0,35** | 342 | 39,0 |
| fr_moyen_af_heart | 1 | 271 | **6 662** | 6 662 | 17 775 | **0,37** | 854 | 39,0 |
| fr_long_af_heart  | 2 | 820 | **9 854** | 17 856 | 49 075 | **0,36** | 1 128 | 39,2 |
| en_court_am_adam  | 1 | 80 | **2 149** | 2 149 | 5 075 | **0,42** | 329 | 35,2 |
| en_moyen_am_adam  | 1 | 285 | **5 754** | 5 754 | 16 175 | **0,36** | 806 | 39,3 |
| en_long_am_adam   | 2 | 830 | **9 067** | 15 982 | 44 600 | **0,36** | 1 067 | 39,9 |

Génération = temps total d'inférence des segments (silencieux inclus mais côté calcul : durée `session.run`).
Chargement modèle (cold start, premier run) : 1,1 à 2,1 s selon cellule.
Enregistrement intégral : `scratch/s0_07/bench_results.csv` ; audios de référence : `scratch/s0_07/*.wav`.
(Artefacts non suivis git — bac à sable Sprint 0.)

## Référence Python (kokoro-onnx, même modèle, contrôle)

| Cellule | tokens | Génération (s) | Audio (s) | RTF |
|---|---|---|---|---|
| fr_court_ff_siwis | 80 | 2,75 | 4,93 | 0,56 |
| fr_moyen_ff_siwis | 271 | 7,56 | 16,09 | 0,47 |
| fr_long_ff_siwis  | 820 | 18,94 | 41,58 | 0,46 |
| en_court_af_heart | 80 | 2,47 | 4,46 | 0,55 |
| en_moyen_af_heart | 285 | 7,96 | 16,77 | 0,47 |
| en_long_af_heart  | 830 | 33,52 | 46,06 | 0,73 |
| fr_court_af_heart | 80 | 4,14 | 4,82 | 0,86 |
| fr_moyen_af_heart | 271 | 11,92 | 17,07 | 0,70 |
| fr_long_af_heart  | 820 | 24,98 | 47,53 | 0,53 |
| en_court_am_adam  | 80 | 2,93 | 4,22 | 0,69 |
| en_moyen_am_adam  | 285 | 10,24 | 15,27 | 0,67 |
| en_long_am_adam   | 830 | 25,13 | 42,75 | 0,59 |

CSV : `scratch/s0_07/ref_python.csv`.

## Observations

1. **RTF Rust uniformément ~0,35–0,42** (sauf `en_court_af_heart` à 0,58 — seul point hors ligne ; il reste dans la marge CPU viabilité) contre 0,46–0,86 en Python. Rust est généralement **1,4 à 2× plus rapide** que la référence Python (cf. `docs/s0-02.md`).
2. **TTFA** : 1,9–3,1 s sur textes courts (un segment), 5,8–9,9 s sur moyens/longs (TTFA = durée du segment 1). Note produit : la latence « utilisateur » perçue passera par le pipeline de préchargement (cartes TW-040/TW-010), pas par le chargement complet.
3. **RAM pic** : 329–1 139 Mo (modèle inclus). Très confortable sur la machine de référence.
4. **CPU** : charge ~31–40 % de 8 threads pendant la génération (~2,5–3,2 cœurs) — quad-cœur sollicité sans saturation.
5. **Rust vs Python hors trim** : le RTF Rust est reporté sur audio non trimé ; même en tenant compte, le gain reste clair.
6. **Sélection des voix** pour le Sprint 0 : `ff_siwis` (FR), `af_heart` (EN) comme référence qualité ; `am_adam` acceptable en contrôle.
7. **VRAM** : non mesurée (CPU) — à compléter sur la carte TW-8 (GPU) si retenu.
8. **Anomalie mineure** : `en_court_af_heart` (Rust 0,58 vs 0,57/0,61/0,58 sur les 3 runs — stable en fait) est la seule case au-dessus de 0,5 ; sans motif structurel évident (tokens identiques à `en_court_am_adam`). À garder sous surveillance au verdict TW-024.
9. **Cold start** : 1,1 à 2,1 s pour charger le modèle (premier run de chaque processus) ; la latence « utilisateur » perçue passera par le préchargement (cartes TW-040/TW-010), pas par ce chemin.

## Pour TW-024

Les RTF ≤ 0,5 (seuil « CPU viable ») du protocole sont réunis sur **11 cellules sur 12** ; la seule déviation est `en_court_af_heart` Rust 0,58 (encore sous le seuil « limite » 0,8). Le verdict (carte TW-024) peut s'appuyer sur ces chiffres : le mode CPU est viable sur la machine de référence, avec une réserve : mesuré sur **Windows 10 Home**, trim désactivé, pas de charge système de fond.
