# S1 — Priorisation du backlog Sprint 1 sur résultats réels (carte TW-030)

Date : 2026-11-18. Entrées : `docs/s0-09b-criteres-reussite.md`, `docs/s0-07c-cpu-verdict.md`, `docs/decisions.md`, `BACKLOG.md` (Sprint 1 provisoire), `.atlas/board.json`.
Cadre : « Si un risque majeur subsiste, le traiter avant toute fonctionnalité. »
Verdict Sprint 0 : **go conditionnel** — aucun risque technique rédhibitoire, mais 17 incertitudes tracées.

## Résultats réels qui pilotent la priorisation

| Constat (preuve) | Conséquence pour le Sprint 1 |
|---|---|
| RTF médian 0,36 ; 11/12 cellules ≤ 0,5 ; `en_court_af_heart` 0,58 « limite » (`docs/s0-07c-cpu-verdict.md`) | CPU viable, marge correcte ; garder le préchargement (N-040) et la surveillance RTF ; GPU optionnel, pas un prérequis (N-008 déclassée en fin de sprint, si capacité) |
| Pause/reprise/arrêt jamais vérifiés en intégration ; file jamais exercée sur ≥ 2 chunks (critère 3, `s0-09b`) | **Risque produit majeur n°1 → traité avant toute fonctionnalité (N-020)** |
| Restauration presse-papiers non validée ; capture S0 écrase le presse-papiers (critère 4, incertitude 4) | **Risque données utilisateur → N-043 (porte l'abandon explicite de N-013) prioritaire** |
| Cas limites de capture non couverts (incertitude 5) | **Risque UX majeur → N-014** |
| Démo S0 partielle : pas de binaire unique, pas d'UI (incertitude 10) ; prototypes hors `src-tauri/` | **Intégration avant ajout** : fusionner les prototypes dans les modules Rust (N-031 fait) avant toute fonctionnalité nouvelle |
| `TtsEngine` non esquissé (incertitude 11) ; Kokoro couplé aux prototypes | **N-009 avant N-039** — exigence d'architecture, coût faible (S) |
| Mesures sur Windows 10 Home, cible produit Windows 11 (incertitude 2) | **Décision humaine requise** avant toute livraison ; re-valider sur Win 11 ou acter Win 10 comme machine de référence |
| Restauration/repli UI Automation non tranchés (incertitude 7) | **N-015 en décision go/no-go en début de sprint** ; N-044 exécutée seulement si retenu |
| Discordance raccourcis Ctrl+Alt+Espace (backlog) vs Ctrl+Shift+R/P/S (prototypes) (critère 5) | **À trancher en début de sprint** (décision humaine ou carte N-018) |
| Cold start 1,1–2,1 s (incertitude 16) | N-040 (préchargement) — à planifier tôt dans l'intégration |
| `ff_siwis` / `fr_siwis` divergence doc benchmark (incertitude 17) | Sans incidence ; documentation déjà corrigée en `s0-09b` |

## Règle d'ordre retenue

**Phase 0 — risques majeurs d'abord** → **Phase 1 — intégration verticale (cœur produit)** → **Phase 2 — présence minimale (tray/panneau/icônes)** → **Phase 3 — robustesse et distribution**.
Une carte n'est pas commencée avant que les cartes dont elle dépend soient closes ou explicitement arbitrées.

## Phase 0 — Risques majeurs (bloquants, avant toute fonctionnalité)

| # | Carte | Prio/Taille | Pourquoi d'abord |
|---|---|---|---|
| 1 | **N-020** — Valider pause, reprise, arrêt et gestion de la file | P1 / M | Écart critère 3 ; seul test d'intégration sur ≥ 2 chunks jamais fait ; rodio validé en labo, pas en chaîne complète |
| 2 | **N-014** — Gérer les cas limites de capture | P1 / M | Écart critère 4 ; aucune sélection, copie asynchrone, app bloquante, interruption — aucun message clair produit à ce jour |
| 3 | **N-043** — Durcir la capture presse-papiers (formats, restauration garantie) | P1 / L | Porte la restauration N-013 reportée ; risque de perte de données utilisateur ; capture actuelle écrase le presse-papiers |
| 4 | **N-009** — Esquisser `TtsEngine` | P2 / S | Exigence d'architecture (isolation Kokoro) ; dépendance directe de N-039 ; coût très faible |
| 5 | **N-015** — Évaluer UI Automation / TextPattern (go/no-go) | P2 / M | Décide si N-044 entre au sprint ; sans décision, la capture dépend uniquement du presse-papiers |
| 6 | **N-018** — Tester les raccourcis proposés et leurs conflits | P2 / S | Tranche la discordance Ctrl+Alt+Espace vs Ctrl+Shift+… ; nécessaire avant N-045 |
| 7 | **N-021** — Tester les déclenchements rapides et répétés | P2 / S | Stop pendant génération, relance pendant lecture ; complète N-020 |

Décisions humaines à recueillir **avant ou pendant la Phase 0** : Win 10/Win 11 (machine de référence) · raccourcis retenus · N-015 go/no-go · N-008 (GPU) tenté ou non.

## Phase 1 — Intégration verticale (cœur produit)

| # | Carte | Prio/Taille | Dépendances |
|---|---|---|---|
| 8 | **N-032** — Machine d'états de lecture (+ événements UI) | P1 / M | Modules Rust (N-031, fait) |
| 9 | **N-035** — Nettoyage/normalisation du texte capturé | P1 / M | — |
| 10 | **N-036** — Détection automatique FR/EN | P1 / M | N-035 |
| 11 | **N-037** — Segmentation en production + file | P1 / M | N-010 (proto), N-035, N-036 |
| 12 | **N-038** — Tests unitaires nettoyage/segmentation/langue | P2 / M | N-035..037 |
| 13 | **N-039** — Intégrer Kokoro derrière `TtsEngine` (chargement en tâche de fond) | P1 / L | N-009, N-032 |
| 14 | **N-040** — Précharger les segments et régler la taille de segment | P1 / M | N-039 ; benchmarks `s0-07b/c` |
| 15 | **N-041** — Intégrer lecture/pause/reprise/arrêt à la machine d'états | P1 / M | N-020, N-032, N-039 |
| 16 | **N-045** — Brancher les raccourcis globaux (lire/pause-reprise/arrêt) | P1 / M | N-018, N-032, N-041 ; signaler l'échec d'enregistrement |
| 17 | **N-033** — Instance unique + démarrage résident sans fenêtre | P1 / M | N-032 |
| 18 | **N-034** — Journalisation locale + erreurs centralisées (sans télémétrie) | P2 / S | — |
| 19 | **N-046** — Messages clairs quand la capture échoue | P2 / S | N-014 |
| 20 | **N-016** — Panel d'applications + rapport de compatibilité | P1 / M | N-043 ; alimente `docs/limites-compat` (N-064) |

## Phase 2 — Présence minimale (décision UI figée : panneau A + anneaux C, sans pastille B)

| # | Carte | Prio/Taille | Dépendances |
|---|---|---|---|
| 21 | **N-069** — Icône .ico multi-tailles + lisibilité 16-24 px | P1 / S | logo source |
| 22 | **N-047** — Icône de zone de notification + menu (lire/pause/arrêt/paramètres/quitter) | P1 / M | N-032 |
| 23 | **N-050** — Anneaux d'état de l'icône (prêt/génération/lecture/pause/erreur) | P1 / M | N-032, N-047 |
| 24 | **N-048** — Panneau ancré à l'icône (maquette A) | P1 / L | N-032, N-047 ; pas de mini-lecteur |

## Phase 3 — Robustesse, qualité, distribution

| # | Carte | Prio/Taille | Notes |
|---|---|---|---|
| 25 | **N-062** — Tests (capture, file audio, machine d'états) + CI | P1 / M | Peut démarrer en Phase 1, à finir ici |
| 26 | **N-042** — Gestion du périphérique audio par défaut | P2 / S | rodio/cpal |
| 27 | **N-064** — Documenter les limites de compatibilité | P2 / S | Après N-016 |
| 28 | **N-008** — Tester l'accélération GPU (si chaîne disponible) | P2 / M | Optionnel ; CPU viable ; sinon documenter |
| 29 | **N-059 / N-060 / N-061** — Gestion modèles, installeur Windows, mentions légales | P1 / M·M·S | `cargo tree`/`cargo about` requis avant distribution (incertitude 15) ; en fin de sprint ou Sprint 2 selon capacité |

## Sprint 2+ (inchangé, non priorisé ici)

N-049 (panneau abouti multi-écrans/DPI), N-051/052/053/054 (paramètres, raccourcis configurables, voix/vitesse/volume, lancement Windows), N-055..058 (signature sonore), N-063 (déchargement modèle), N-065 (accessibilité), N-066 (install/maj modèles auto), N-067 (Piper/MeloTTS), N-013 (clôture restauration, si non absorbée par N-043).

## Hors Sprint 1 / à acter

- **N-013** (restauration multi-formats) : traitée **dans N-043** ; la carte N-013 reste close « exploré, reportée » — ne pas la rouvrir.
- **N-044** (repli UI Automation) : entre au sprint **uniquement si N-015 retient la piste**.
- **N-067** (moteur alternatif) : seulement si Kokoro échoue sur latence/CPU/licence — aucun signal en ce sens (RTF 0,36).
- **§11 du docx source** : contrôle effectué sur reconstruction confirmée par l'humain ; à relire si le document est retrouvé (incertitude 1).
- **`ff_siwis` / `fr_siwis`** : documentation alignée, sans action.

## Critère de fin de Sprint 1 (M1)

1. Les 3 risques majeurs (N-020, N-014, N-043) clos avec preuves d'intégration.
2. Lecture de bout en bout sur un texte > 5 segments : raccourci → capture → nettoyage → langue → segmentation → Kokoro → audio progressif → pause/reprise/arrêt, dans l'application Tauri (pas de prototype isolé).
3. Icône tray + menu opérationnels ; panneau A minimal ou explicitement reporté tracé.
4. Décision Win 10/Win 11 actée ; rapport de compatibilité (N-016) publié.
5. Aucune régression benchmark RTF > 0,5 médiane ; `en_court_af_heart` surveillé (incertitude 13).

## Journal de décisions

Entrée ajoutée à `docs/decisions.md` (TW-030) : ordre Phase 0→Phase 3, risques majeurs avant fonctionnalités, GPU optionnel, UI Automation soumise à go/no-go.

## Traçabilité

- Carte : TW-030. Sources : `docs/s0-09b-criteres-reussite.md`, `docs/s0-07a/b/c`, `docs/decisions.md`, `BACKLOG.md`, `.atlas/board.json`.
- Cartes concernées : N-008, N-009, N-013…N-069 (voir phases ci-dessus).
- Ce document est proposé en PR ; la fusion et la mise à jour définitive de `main` restent à l'humain (R6).
