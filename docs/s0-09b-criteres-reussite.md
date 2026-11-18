# S0-09b — Vérification des critères de réussite du Sprint 0 (carte TW-028)

Date : 2026-11-18. Statut : **contrôle effectué — verdict « go conditionnel »** (détails ci-dessous).
Machine : `docs/environnement-reference.md` — Windows 10 Home 10.0.19045, i7-7700K (4C/8T), 32 Go, RTX 3060 Ti 8 Go (driver 596.49, **non utilisée**). « mesuré sur Windows 10 Home ».

## Note méthodologique (R9)

Les « 8 critères du doc §11 » référencent `Navi_Synthese_Projet_Sprint_0.docx`, **absent du dépôt, de l'historique git et de toute branche** (vérifié le 2026-11-18 : aucun `.docx` dans le repo ; recherche disque sur `D:\Git\Projets` sans résultat ; `git log --all --diff-filter=D --name-only -- '*.docx'` vide). Contourner en silence est exclu (R9) : l'humain a confirmé (2026-11-18) la **reconstruction des 8 critères à partir des 8 epics du Sprint 0 du `BACKLOG.md`** (EP-00 Mise en place → EP-07 Clôture). Si le docx est retrouvé, ce contrôle devra être relu contre le vrai §11.

Correspondance retenue : **1 epic = 1 critère** ; le critère est « atteint » si les tâches P0 de l'epic sont réalisées **avec preuves consignées**, « atteint avec réserves » si la validation existe mais qu'un écart tracé subsiste, « partiel » si un pan entier manque.

## Les 8 critères contrôlés

| # | Critère (epic) | Cartes | Statut | Preuves |
|---|---|---|---|---|
| 1 | **EP-00 Mise en place** — squelette qui compile/lance, journal de décisions, fiche environnement | N-001, N-002, N-003 (done) | ✅ **Atteint** | `README.md`, `docs/decisions.md`, `docs/environnement-reference.md` ; squelette Tauri 2 + React/TS, `cargo check` OK ; divergence OS signalée (Win 10 vs Win 11) |
| 2 | **EP-01 Moteur TTS** — artefacts Kokoro récupérés, échantillons FR/EN de qualité acceptable, inférence CPU depuis Rust, phonémisation FR/EN résolue | N-004, N-005, N-006, N-007 (done) ; N-008, N-009 (todo) | ✅ **Atteint** (réserves mineures) | `docs/s0-02.md` (ort 2.0.0-rc.15, RTF 0,38, chargement 1,29 s) ; échantillons FR/EN audités à l'oreille (af_heart / am_adam retenues, N-005/N-029) ; phonémisation espeak-ng (tokens identiques à la référence Python) |
| 3 | **EP-02 Lecture progressive et audio** — segmentation en phrases sans coupure arbitraire ; 1er segment audible avant la fin de la génération totale ; délai avant 1re parole mesuré ; bibliothèque audio validée | N-010, N-011, N-019 (done) ; N-020 (todo) | ⚠️ **Atteint avec réserves** | `docs/s0-10.md` (segmentation FR/EN, chunks ≤ 480 tokens) ; `docs/s0-11.md` (chaîne texte→rodio, RTF 0,47) ; `docs/s0-19.md` (rodio 5/5 tests) ; **mais** pause/reprise/arrêt « sans perte de position » jamais vérifiés en intégration (N-020), file producteur/consommateur jamais exercée sur ≥ 2 chunks (tous les tests S0 ≤ 1 chunk) |
| 4 | **EP-03 Capture de sélection** — capture par presse-papiers ; restauration multi-formats ; cas limites ; rapport de compatibilité | N-012 (done) ; N-013 (done, exploré) ; N-014, N-015, N-016 (todo) | ⚠️ **Partiel** | `docs/s0-12.md` (WM_COPY → énumération formats → UTF-16/ANSI/HTML/RTF, 176 chars réels) ; `docs/s0-13.md` (restauration non validée : cast HGLOBAL→HANDLE, reportée Sprint 1, **délibéré et tracé**) ; cas limites (N-014), repli UI Automation (N-015), panel d'applications (N-016) **non couverts** |
| 5 | **EP-04 Raccourcis globaux** — enregistrement hors focus, détection d'échec, conflits documentés | N-017 (done) ; N-018 (todo) | ✅ **Atteint** (réserve P2) | `docs/s0-17.md` (global-hotkey 0.3, Ctrl+Shift+R/P/S enregistrés sans conflit, événementiel OK) ; conflits et signalement d'échec (Ctrl+Alt+Espace/P/S) non testés (N-018) ; à noter : le backlog mentionne Ctrl+Alt+Espace, les prototypes ont utilisé Ctrl+Shift+R/P/S — discordance assumée, à trancher en Sprint 1 |
| 6 | **EP-05 Benchmarks** — latence avant 1re parole, RTF, RAM/VRAM, charge CPU/GPU, verdict mode CPU | N-022, N-023, N-024 (done) ; N-008 (todo, P2) | ✅ **Atteint** | `docs/s0-07a/b/c` (protocole reproductible ; RTF médiane **0,36**, 11/12 cellules ≤ 0,5, aucune > 0,8 — seule déviation `en_court_af_heart` 0,58 « limite » ; RAM pic 329–1 139 Mo ; CPU ~31–40 % ; TTFA 1,9–9,9 s ; cold start 1,1–2,1 s) ; **réserve : mesuré sur Windows 10 Home, audio Rust non trimé, corpus synthétique** ; GPU non testé (optionnel, CPU viable) |
| 7 | **EP-06 Licences** — audit moteur/poids/voix/phonémiseur/dépendances ; conditions de redistribution | N-025, N-026 (done) | ✅ **Atteint** | `docs/licences.md` (passe 2, licences brutes lues) : projet **GPL-3.0-or-later**, Kokoro Apache-2.0, espeak-ng GPL-3.0 embarquable, `ff_siwis` (CC BY) hors distribution, Tauri sans logo CC-BY-NC-ND ; redistribution via installeur embarqué (~110–140 Mo) ; `cargo tree`/`about` reporté (TW-60/TW-61) |
| 8 | **EP-07 Clôture** — démo intégrée ; architecture validée et voix/libs finales ; backlog Sprint 1 priorisé | N-027, N-029 (done) ; N-028 (doing), N-030 (todo) | ⚠️ **En cours** | `docs/s0-09a-demo.md` (démo **partielle**, délibérée : TTS ré-exécutée et audible — Kokoro chargé en 10,98 s, 296 tokens, inférence 22,7 s ; capture/raccourcis/pause validés par prototypes uniquement ; **pas de binaire unique, pas d'UI**) ; architecture validée (N-029) ; **priorisation Sprint 1 non faite (N-030)** |

**Synthèse : 4/8 atteints pleinement (1, 2, 5, 6) — 2 atteints avec réserves tracées (3, 7) — 1 partiel (4) — 1 en cours (8).** Aucun critère n'est « échoué » ; les écarts sont connus, consignés, et majoritairement délibérés (Sprint 0 = validation, pas finition).

## Incertitudes restantes (à lever ou à acter)

1. **§11 source manquant** (docx absent) — le présent contrôle est une reconstruction confirmée par l'humain ; à relire si le document est retrouvé.
2. **Divergence OS** (R9) : toutes les mesures sont sous Windows 10 Home ; la cible produit est Windows 11 (WinRT/`windows` crate, tray, Tauri 2). À re-valider sur Win 11 ou à acter comme « machine de référence officielle » — décision humaine attendue.
3. **Pause/reprise/arrêt « sans perte de position ni désynchronisation »** jamais vérifiés hors isolation (carte N-020, todo) — rodio est validé en laboratoire, pas dans la chaîne complète.
4. **Restauration du presse-papiers** non validée (carte N-013 close « exploré, reporté Sprint 1 ») — la capture S0 **écrase le presse-papiers utilisateur**, risque UX/données connu et accepté pour S0 uniquement.
5. **Cas limites de capture** non couverts (N-014) : aucune sélection, délai de copie asynchrone, application qui bloque la copie, interruption — aucun message d'erreur clair produit à ce jour.
6. **Panel de compatibilité applications** (N-016) absent : navigateurs, Word, éditeur, PDF, terminal, Electron, apps élevées — la compatibilité réelle est inconnue hors Chromium.
7. **Repli UI Automation / TextPattern** non tranché (N-015) — piste morte ou retenue ?
8. **Conflits de raccourcis et signalement d'échec** non testés (N-018).
9. **Lecture progressive multi-chunks** jamais démontrée avec audio en lecture pendant la génération du chunk suivant (tests S0 ≤ 1 chunk ; benchmarks longs = 2 chunks en inférence séquentielle, pas en pipeline progressif démontré).
10. **Démo S0-09a partielle** (décision tracée) : aucun binaire unique, aucune intégration Tauri, aucune UI ; la « validation » repose sur des prototypes isolés dans `scratch/` (non suivis git).
11. **`TtsEngine` (isolation du moteur) non esquissé** (N-009, P1/S) — exigence d'architecture non démontrée ; Kokoro reste couplé aux prototypes.
12. **Voix masculine EN `am_adam`** notée F+ en amont (validée à l'oreille ; alternatives `am_michael`/`am_fenrir`/`am_puck` en secours, N-029).
13. **`en_court_af_heart` RTF 0,58** (zone « limite ») — sous surveillance au prochain benchmark.
14. **Provenance des données d'entraînement des voix EN** non documentée amont — notée, non bloquante.
15. **`cargo tree` / `cargo about`** sur les dépendances finales — reporté à l'installeur (TW-60/TW-61), requis avant distribution.
16. **Cold start 1,1–2,1 s** — à masquer par préchargement du modèle (décision tracée, TW-010/TW-040, Sprint 1).
17. **`ff_siwis` nommée `fr_siwis` dans le protocole de benchmark mergé** — documenté, sans incidence sur les verdicts.

## Verdict et recommandations

**« Go conditionnel » pour le Sprint 1**, sous réserve de :

1. **Priorité Sprint 1 bloquante** : N-020 (pause/reprise/arrêt sans perte de position) et N-014 (cas limites de capture) — les deux risques produit majeurs non couverts ;
2. **Acter ou lever la divergence Win 10/Win 11** (décision humaine) avant toute livraison ;
3. **Prioriser le backlog Sprint 1** (carte N-030) — le jalon M0 n'est pas clos sans cette étape ;
4. **Intégrer avant d'ajouter** : fusionner les prototypes dans `src-tauri/` (modules `capture`, `hotkey`, `tts`, `audio`, `state`) avant toute fonctionnalité nouvelle ;
5. **Restauration presse-papiers** à traiter en Sprint 1 (N-013 reportée) — risque données utilisateur ;
6. **Jalon M0** non clos tant que N-028 (ce contrôle, PR en cours) et N-030 sont ouvertes.

Ce contrôle ne vaut pas « tous les voyants sont verts » : il vaut **« aucun risque technique rédhibitoire identifié ; la chaîne Kokoro locale FR/EN est validée avec preuves ; les écarts restants sont tracés et planifiés »**. La décision go/no-go finale appartient à l'humain lors de la revue de fin de Sprint 0.

## Traçabilité

- Preuves : `docs/s0-02.md`, `docs/s0-10.md`, `docs/s0-11.md`, `docs/s0-12.md`, `docs/s0-13.md`, `docs/s0-17.md`, `docs/s0-19.md`, `docs/s0-07a-benchmark-protocol.md`, `docs/s0-07b-measurement.md`, `docs/s0-07c-cpu-verdict.md`, `docs/s0-09a-demo.md`, `docs/licences.md`, `docs/environnement-reference.md`, `docs/decisions.md`.
- Cartes liées : N-004…N-030 (voir `.atlas/board.json`) ; décisions consignées dans `docs/decisions.md` (verdict CPU, voix, interface, licence).
- La clôture de la carte (`atlas-kanban done 28`) intervient après fusion de la PR (merge = humain, R6).
