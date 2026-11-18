# Journal de décisions - Navi

Projet : Navi - Lecteur de texte vers parole
Stack : Rust + Tauri 2 + React/TypeScript, Windows 11

---

## Décision initiale : Création du journal de décisions (TW-002, S0-00b)

**Date** : 2026-11-17
**Contexte** : Cartographie des décisions initiales pour le sprint 0, en amont de toute implémentation.
**Décision** : Créer un journal de décisions centralisé `docs/decisions.md` pour consigner les choix et les compromis au fil de l'eau.
**Implication** : Chaque carte documentera sa décision clé dans ce fichier.
**Raison** : Permet la traçabilité temporelle et la cohérence entre les décisions, avec le tableau vivant comme source de vérité.

---

## Décision : Périphérique audio par défaut (TW-??)

**Date** : 2026-11-17
**Contexte** : Gestion du périphérique audio par défaut pour la lecture TTS.
**Décision** : Implémenter la gestion du périphérique audio par défaut avec détection de changement et d'absence de périphérique.
**Implication** : Nécessite des vérifications à l'exécution et une gestion de repli.
**Raison** : Assure une expérience utilisateur robuste indépendamment du matériel de sortie configuré par l'utilisateur.

---

## Décision : Interface avec icône uniquement (TW-68)

**Date** : 2026-11-16
**Contexte** : Scénario de présence minimale pour Navi.
**Décision** : Interface avec panneau ancré à l'icône de notification uniquement (panneau A + anneaux d'état C), sans mini-lecteur flottant.
**Implication** : Simplifie UX, réduit pollution écran, met l'accent sur les contrôles essentiels lire/pause/stop.
**Raison** : Réponse à l'utilisation typique d'agrandissement de icône, sans surcharge visuelle ; satisfait les premiers retours utilisateurs.

---

## Décision : Filtre CPU contre GPU pour ONNX Runtime (TW-??)

**Date** : 2026-11-17
**Contexte** : Chaîne de génération de voix Kokoro-82M, maximisant la stabilité de déploiement.
**Décision** : Environnement uniquement CPU DirectML (ONNX Runtime), rejetant la chaîne GPU sauf si reproductible.
**Implication** : Garantit une détection de compatibilité plus rapide et élimine la maintenance pilote.
**Raison** : Priorise la stabilité et la compatibilité sur une optimisation brute ; se réserve le GPU pour une chaîne davantage standardisée.

---

## Décision : Bibliothèque audio Rust (TW-??)

**Date** : 2026-11-17
**Contexte** : Bibliothèque audio pour la lecture, les transitions, la pause/reprise.
**Décision** : Crate `rodio`/`cpal` pour lecture en continu, transitions sans silence.
**Implication** : Version minimale de `rodio` avec appairage de périphérique, sans effets.
**Raison** : Bibliothèque Rust mature, bibliothèque AV multiplateforme, n'exigeant pas de dépendances CUDA externes.

---

## Décision : Raccourcis globaux (TW-??)

**Date** : 2026-11-17
**Contexte** : Éléments d'interface en raccourcis pour lire/pause-reprise/stop.
**Décision** : Plugin Tauri `global-shortcut` avecallbacks hors focus.
**Implication** : `global-shortcut` plutôt que `RegisterHotKey` pour Tauri 2.
**Raison** : Plugins intégrés pour hotkeys de fenêtre, gestion native des combinaisons, events hors focus ; plus stable à long terme.

---

## Décision : Modèle de machine d'états (TW-??)

**Date** : 2026-11-17
**Contexte** : État global de lecture et capture, cohérence UI.
**Décision** : Simple machine d'états unifiante : inactif / capture / génération / lecture / pause / erreur, avec émission d'événements vers React UI.
**Implication** : Contrôle centralisé, validation d'état possible, réinitialisation facile.
**Raison** : Patte de sortie du backend stable pour toute l'application, limite d'état minimal pour mémoire des erreurs plus facile.

---

## Décision : Fourniture de texte (TW-??)

**Date** : 2026-11-17
**Contexte** : Bibliothèque de fourniture de texte pour accès contrôle par logiciels.
**Décision** : Bibliothèque `WinRT` native (`ISelectionProvider`) contre `UI Automation` pour texte enrichi + révélation obligatoire de focus.
**Implication** : Détection explicite de focus de fenêtre, interopérabilité plus riche.
**Raison** : Plus de texte complet, plus de cache, UI Automation TF plus une fragile, notre focus `WinRT` sur `ISelectionProvider`.

---

## Décision : Signature sonore (TW-55)

**Date** : letownization dans la carte TW-55, la signature originale `hey_listen!` est maintenant enregistrée.
**Contexte** : Signature de démarrage Windows personnalisée, distincte, non extraite.
**Décision** : Enregistrer une boucle de 0,8-1,5s à voix aiguë, volume modéré, pas d'imitation d'une franchise existante.
**Implication** : Enregistrement, traitement WAV, lecture unique après raccourcis prêts.
**Raison** : Identité de marque mémorable, brevetée, avec fatigue minimal sur longue durée d'utilisation.

---

## Décision : Linting et formatage (TW-??)

**Date** : 2026-11-17
**Contexte** : Cohérence code entre Rust et TypeScript.
**Décision** : `rustfmt`/`clippy` pour Rust ; `prettier`/`eslint` pour TS ; préengagement `format --verify`.
**Implication : Garantit un code propre, limite linting manuel.
**Raison** : Contrôle qualité à long terme, automatise le débordement des hooks.

---

## Décision : Mode CPU viable pour le Sprint 0 (TW-024, S0-07c)

**Date** : 2026-11-17
**Contexte** : Verdict de viabilité après la campagne de benchmark Rust EP CPU + contrôle Python (TW-022/TW-023, `docs/s0-07a-benchmark-protocol.md`, `docs/s0-07b-measurement.md`, `docs/s0-07c-cpu-verdict.md`).
**Décision** : Le mode CPU est **viable pour le Sprint 0** sur la machine de référence ; la chaîne produit repose sur ONNX Runtime CPU (`ort` 2.0.0-rc.15). Le GPU (cartes TW-8/S0-02c) devient un gain optionnel, pas un prérequis. Préchargement du modèle à l'inactivité/au démarrage planifié (carte TW-010) pour éviter le cold start utilisateur.
**Implication** : Verdict = go CPU pour la démo Sprint 0 (TW-027) et la validation des critères de réussite (TW-028) ; `en_court_af_heart` (RTF 0,58) reste sous surveillance au prochain benchmark ; VRAM non applicable mesurée ; réserve « Windows 10 Home » à lever en re-validant sur Win 11 si possible.
**Raison** : RTF médian 0,36 (11 cellules ≤ 0,5 sur 12 ; la déviation `en_court_af_heart` 0,58 reste « limite », aucun cas > 0,8) ; RAM pic 329–1 139 Mo (modèle inclus) ; CPU ~31–40 % de 8 threads ; marge CPU confortable en cohérence avec `docs/s0-02.md` (0,38).

---

## Décision : Priorisation du Sprint 1 sur résultats réels (TW-030, S0-09c)

**Date** : 2026-11-18
**Contexte** : Le Sprint 0 se clôt sur un verdict « go conditionnel » (`docs/s0-09b-criteres-reussite.md`) : chaîne Kokoro CPU validée (RTF médian 0,36), mais trois risques majeurs non couverts — pause/reprise/arrêt en intégration (N-020), restauration presse-papiers (N-013→N-043), cas limites de capture (N-014).
**Décision** : Ordre Sprint 1 en quatre phases — Phase 0 risques majeurs (N-020, N-014, N-043, N-009, N-015, N-018, N-021), Phase 1 intégration verticale (N-032, N-035..N-041, N-045, N-033, N-034, N-046, N-016), Phase 2 présence minimale (N-069, N-047, N-050, N-048), Phase 3 robustesse/distribution (N-062, N-042, N-064, N-008 optionnel, N-059/060/061). GPU optionnel (CPU viable) ; UI Automation soumise à go/no-go (N-015) ; N-013 absorbée par N-043. Détail : `docs/s1-priorisation.md`.
**Implication** : Aucune fonctionnalité nouvelle avant la clôture des risques Phase 0 ; intégration des prototypes dans `src-tauri/` avant tout ajout ; décisions humaines attendues : Win 10/Win 11, raccourcis, N-015, N-008.
**Raison** : « Si un risque majeur subsiste, le traiter avant toute fonctionnalité » (carte TW-030) ; les écarts restants sont connus et tracés, pas de risque rédhibitoire identifié.

---

## Prochaine entrée de décision à consigner

**Date à venir** : Suivre toute décision dans un sprint ultérieur.

---