# S0-07c — Tableau de benchmarks et verdict mode CPU (carte TW-024)

Date : 2026-11-17. Statut : **verdict : CPU viable** — appuyé sur `docs/s0-07b-measurement.md` (TW-023) et le protocole `docs/s0-07a-benchmark-protocol.md` (TW-022).
Machine : machine de référence — Windows 10 Home 10.0.19045, i7-7700K (4C/8T), 31,9 Go, RTX 3060 Ti 8 Go (driver 596.49, **non utilisée**), commit `ed99346`. « mesuré sur Windows 10 Home ».

## Tableau consolidé — EP CPU (Rust, `ort` 2.0.0-rc.15), médiane des 3 runs

| Cellule | segments | tokens | TTFA (ms) | Génération (ms) | RTF | RAM pic (Mo) | CPU % | Seuil protocole |
|---|---|---|---|---|---|---|---|---|
| fr_court_ff_siwis | 1 | 80 | 1 980 | 1 980 | **0,36** | 345 | 39,5 | ✅ viable |
| fr_moyen_ff_siwis | 1 | 271 | 6 111 | 6 111 | **0,37** | 816 | 39,3 | ✅ viable |
| fr_long_ff_siwis  | 2 | 820 | 8 835 | 17 076 | **0,40** | 1 016 | 39,3 | ✅ viable |
| en_court_af_heart | 1 | 80 | 2 961 | 2 961 | **0,58** | 330 | 31,5 | ⚠️ limite |
| en_moyen_af_heart | 1 | 285 | 6 179 | 6 179 | **0,35** | 841 | 39,1 | ✅ viable |
| en_long_af_heart  | 2 | 830 | 9 612 | 16 960 | **0,36** | 1 134 | 40,4 | ✅ viable |
| fr_court_af_heart | 1 | 80 | 1 921 | 1 921 | **0,35** | 342 | 39,0 | ✅ viable |
| fr_moyen_af_heart | 1 | 271 | 6 662 | 6 662 | **0,37** | 854 | 39,0 | ✅ viable |
| fr_long_af_heart  | 2 | 820 | 9 854 | 17 856 | **0,36** | 1 128 | 39,2 | ✅ viable |
| en_court_am_adam  | 1 | 80 | 2 149 | 2 149 | **0,42** | 329 | 35,2 | ✅ viable |
| en_moyen_am_adam  | 1 | 285 | 5 754 | 5 754 | **0,36** | 806 | 39,3 | ✅ viable |
| en_long_am_adam   | 2 | 830 | 9 067 | 15 982 | **0,36** | 1 067 | 39,9 | ✅ viable |
| **médiane** | | | **5 924** | | **0,36** | | | |

Lecture des seuils du protocole : ≤ 0,5 viable · 0,5–0,8 limite · > 0,8 non viable.
**VRAM : non applicable** (mesures CPU ; la carte GPU/S0-02c est TW-8).

## Verdict

**Le mode CPU est viable pour le Sprint 0 sur la machine de référence**, avec une marge confortable :

- RTF médian **0,36** ; 11 cellules sur 12 ≤ 0,5 (seuil « CPU viable ») ;
- aucune cellule au-dessus du seuil « non viable » (0,8) ; la seule déviation est `en_court_af_heart` (0,58, stable sur les 3 runs) — à surveiller, pas bloquant ;
- RAM pic 329–1 139 Mo (modèle inclus) sur 31,9 Go — largement supportable ;
- CPU ~31–40 % de 8 threads (~2,5–3,2 cœurs) au moment des générations ;
- TTFA 1,9–9,9 s sur l'audio non trimé ; la latence perçue utilisateur passera par le préchargement (TW-040/TW-010).

## Réserves et limites du verdict

1. **Mesuré sur Windows 10 Home** (cible produit : Windows 11) — divergence à signaler (R9) ; ré-valider dans les conditions Win 11 et premier lancement à froid de l'application finale (carte TW-28).
2. **Audio Rust non trimé** (silences ~2 s conservés) — la RTF réelle sera légèrement **meilleure** en production (trim à la lecture).
3. **Corpus synthétique** (textes de référence du protocole, pas de texte réel chaotique longue durée).
4. **Charge CPU monogénération** : le pipeline multi-segments préchargement n'est pas mesuré ici (produits Sprint 1).
5. `en_court_af_heart` (0,58) à confirmer/réparer sur décision Sprint 1 ; `ff_siwis` nommée `fr_siwis` dans le protocole mergé — documenté, sans incidence.

## Conséquences

- **Sprint 0 peut se poursuivre en CPU** : démo intégrée (TW-027), critères de réussite (TW-028), priorisation Sprint 1 (TW-030).
- **GPU (TW-8 / S0-02c)** reste un plus optionnel ; non bloquant pour la décision CPU de la chaîne produit.
- Cold start ~1,1–2,1 s : à éviter par **préchargement du modèle au démarrage/chargement idle** (carte TW-010) — intégré au backlog Sprint 1.
- La décision est consignée dans `decisions.md` (fenêtre « Mode CPU viable pour le Sprint 0 »).
