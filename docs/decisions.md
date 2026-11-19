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

## Décision : Raccourcis par défaut — Ctrl+Maj+R/P/S (TW-018, S1-018)

**Date** : 2026-11-18
**Contexte** : Discordance S0 (critère 5) entre le backlog initial (Ctrl+Alt+Espace/P/S) et les prototypes Sprint 0 (Ctrl+Shift+R/P/S). Mesure du 2026-11-18 (`cargo run --example hotkey_check`, machine de référence) : Ctrl+Maj+R/P/S enregistrent sans conflit ; **Ctrl+Alt+Espace est déjà réservée** par l'OS ou une application.
**Décision** : Par défaut = **Ctrl+Maj+Espace** (lire), **Ctrl+Maj+P** (pause/reprise), **Ctrl+Maj+S** (arrêt). Ctrl+Alt+Espace est écarté pour « lire » (conflit constaté sur la machine de référence). Amendement du 2026-11-18 sur demande : Espace pour « lire », plus accessible au clavier sans viser une lettre ; Ctrl+Maj+R restait libre mais n'est plus le défaut. En cas de conflit sur une autre machine, l'enregistrement signale l'échec avec un message clair (`RegistrationReport::failure_message`) ; les autres raccourcis restent actifs ; les raccourcis configurables relèvent de N-052.
**Implication** : Le code `hotkey::register_defaults` enregistre indépendamment chaque raccourci et rapporte les échecs (ne panique plus ; échec partiel toléré). Le signalement utilisateur sera rattaché au tray/panneau (N-045/N-052).
**Raison** : Comportement mesuré sur la machine de référence ; uniformité Ctrl+Maj ; détection possible uniquement à l'enregistrement (Windows n'expose pas le détenteur d'une hotkey).

---

## Décision : Ryan voix officielle — migration TTS vers Qwen3-TTS 0.6B (TW-070, N-070, N-071)

**Date** : 2026-11-19
**Contexte** : échantillonnage comparatif de voix (carte N-070), texte Domio identique en FR et EN, Qwen3-TTS-12Hz-0.6B-CustomVoice (Apache-2.0, 0.9 Md params, bf16) testé sur RTX 3060 Ti.
**Décision** : **Ryan** (voix anglaise native, timbre dynamique, compatible FR/EN) est la **voix officielle de Navi**. Le moteur TTS de Navi **migre de Kokoro vers Qwen3-TTS-12Hz-0.6B-CustomVoice** (carte N-071). Kokoro reste en production jusqu'à ce que la migration soit validée terrain (voix, latence, stabilité, recette) ; le moteur de secours reste envisageable (N-067).
**Implications** :
- Qwen3-TTS est un modèle PyTorch/Transformers (pas ONNX) : l'intégration passe par un **sidecar Python** (processus embarqué) ou une **conversion ONNX** non triviale — chantier L, carte N-071 priorité P1.
- Distribution : oblige à embarquer (ou télécharger) torch + poids (~1,8 Go) → revoir mentions légales (N-061) et installeur (N-060) ; modèles et torch non inclus dans le dépôt (gitignorés).
- Licensing : modèle Apache-2.0 (conforme usage produit) mais mentions à intégrer (LICENSE_QWEN / NOTICE) ; vérifier aussi les maths des voix officielles.
- UX : voix préchargée au démarrage de l'app ; latence à mesurer sur lecture réelle (RTF à établir sur machine de référence).
- Kokoro reste utilisé pour les cartes jusqu'à disponibilité Qwen validée (recette humaine avec la voix Ryan sur texte FR et EN).
**Raison** : la décision vient de l'écoute humaine ; Ryan couvre à la fois EN (natif) et FR (lisible) avec un timbre cohérent pour une identité produit unique ; Qwen3-TTS offre 9 voix et une qualité perçue supérieure à Kokoro sur le français.
**Ouvert** : voix par défaut définitive (Ryan), mode fallback si Qwen indisponible (Kokoro), indices de latence/RTF à produire dans N-071, relecture légale lots 2–3 (N-061) avant distribution.

---

## Mise en œuvre : Qwen3-TTS via sidecar Python + repli Kokoro (TW-071, 2026-11-19)

**Date** : 2026-11-19
**Contexte** : exécution de la décision ci-dessus (carte N-071). Détails techniques : `docs/s2-71-migration-qwen3-tts.md`.
**Mise en œuvre** :
- **Sidecar Python persistant** (`src-tauri/resources/qwen_sidecar.py`) : charge le modèle une fois (CUDA bf16 auto, sinon CPU float32 ; `sdpa`, repli `eager`) et sert le contrat TtsEngine via JSONL sur stdio (PCM f32 mono 24 kHz, base64). stdout réservé au protocole ; bannières des libs sur stderr.
- **`QwenTtsEngine`** (`src-tauri/src/tts/qwen.rs`) implémente `TtsEngine` ; voix officielle Ryan par défaut (FR/EN) ; chargement borné (NAVI_QWEN_STARTUP_TIMEOUT_MS, défaut 10 min).
- **`ResilientEngine`** (`src-tauri/src/tts/fallback.rs`) : **Qwen → Kokoro, repli automatique** (panne au chargement ou première synthèse → Kokoro avec message clair). `NAVI_TTS_ENGINE` = `auto` (défaut) | `qwen` | `kokoro`. Kokoro paresseux.
- Sortie 24 kHz mono f32 : la segmentation, la machine d'états et la file audio sont inchangées.
- **RTF mesuré** (RTX 3060 Ti, bf16, sdpa) : FR ≈ 3,0–3,8 ; EN ≈ 3,0–3,5 — la lecture progressive temps réel est **non viable** avec Qwen sur cette machine à ce stade. Kokoro reste donc **le moteur effectif** ; Qwen reste disponible via `NAVI_TTS_ENGINE=qwen` / `auto` pour recette et démo.
**Décision** (2026-11-19, owner) : le défaut `NAVI_TTS_ENGINE=auto` est **Qwen d'abord, repli Kokoro** (chargement ou première synthèse). RTF mesuré ≈ 3 sur RTX 3060 Ti — assumé en l'état pour la recette/démo ; la distribution exigera un RTF ≲ 1 (GPU/flash-attn) ou une validation UX explicite. Recette humaine restante : écoute Ryan, latence perçue, repli Kokoro vérifié.
**Tests** : 78 unitaires Rust (dont 8 sur le protocole sidecar) ; `cargo run --example qwen_check` pour la recette.
**Raison** : exécution de la décision Ryan ; le RTF produit refroidit le calendrier produit mais sécurise l'existant (Kokoro) sans renoncer au moteur cible.

---

## Prochaine entrée de décision à consigner

**Date à venir** : Suivre toute décision dans un sprint ultérieur.

---