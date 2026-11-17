# Navi — Backlog produit

Source : `Navi_Synthese_Projet_Sprint_0.docx`. Stack : Rust + Tauri 2 + React/TypeScript, Windows 11, TTS local (Kokoro-82M / ONNX Runtime en premier choix).

Légende — Priorité : **P0** bloquant / **P1** cœur produit / **P2** important / **P3** souhaitable. Taille : S (≤½ j) · M (1-2 j) · L (3-5 j).
Le Sprint 1 et les suivants sont **conditionnels** aux résultats du Sprint 0 (cf. doc §14) : à repriorisier après la revue de fin de S0.

---

> **Décision UI (2026-11-16)** : panneau A ancré à l’icône + anneaux d’état C ; mini-lecteur flottant abandonné.

## Sprint 0 — Validation technique (expérimentation et décision, pas de finition)

### EP-00 Mise en place
- [ ] **S0-00a** (S, P0) Initialiser le dépôt (`src/`, `src-tauri/`, `models/`, `tests/`, README), squelette Tauri 2 + React/TS qui compile. Ajouter `models/` au `.gitignore`.
- [ ] **S0-00b** (S, P0) Créer le journal de décisions (`docs/decisions.md`) : décisions et compromis consignés au fil de l'eau.
- [ ] **S0-00c** (S, P0) Documenter l'environnement de référence (Win 11, i7-7700K, 32 Go, RTX 3060 Ti 8 Go) ; vérifier qu'aucune dépendance à CUDA Toolkit n'est supposée.

### EP-01 Moteur TTS (S0-01, S0-02)
- [ ] **S0-01a** (M, P0) Récupérer les artefacts Kokoro-82M (modèle ONNX, voix) ; noter source, version, hash, licence.
- [ ] **S0-01b** (M, P0) Générer localement un échantillon **français** et un échantillon **anglais** (WAV) ; comparer 2-3 voix par langue ; noter la qualité subjective.
  - *Validation : 1 phrase FR + 1 phrase EN, qualité jugée acceptable.*
- [ ] **S0-02a** (L, P0) Charger le modèle via ONNX Runtime **depuis Rust** (crate `ort` ou équivalent) ; inférence CPU fonctionnelle.
- [ ] **S0-02b** (M, P0) Résoudre la phonémisation FR/EN (dépendance type espeak-ng / alternative) et évaluer son impact licence + déploiement.
- [ ] **S0-02c** (M, P2) Tester l'accélération GPU (DirectML/CUDA) **uniquement si** la chaîne est disponible et reproductible ; sinon documenter.
- [ ] **S0-02d** (S, P1) Esquisser le trait/interface `TtsEngine` (isolation du moteur) et vérifier qu'il tient pour Piper/MeloTTS.
- [ ] **S0-02e** (M, P3) Évaluer Piper et MeloTTS en réserve (qualité FR, latence, complexité) — seulement si Kokoro échoue sur latence/CPU/licence.

### EP-02 Lecture progressive et audio (S0-03, S0-06)
- [ ] **S0-03a** (M, P0) Prototype de segmentation en phrases / groupes de phrases (sans coupure arbitraire).
- [ ] **S0-03b** (L, P0) Pipeline génération → file ordonnée de segments audio → lecture ; le 1er segment est audible avant la fin de la génération totale.
  - *Validation : mesure du délai « avant première parole » sur un texte long.*
- [ ] **S0-06a** (M, P0) Choisir et valider la bibliothèque audio Rust (ex. `rodio`/`cpal`) : lecture en continu, transitions sans silence artificiel.
- [ ] **S0-06b** (M, P0) Pause / reprise / arrêt sans perte de position ni désynchronisation ; arrêt vide la file et annule proprement les tâches de génération en cours.
- [ ] **S0-06c** (S, P1) Test : déclenchements rapides et répétés (relance pendant lecture, stop pendant génération).

### EP-03 Capture de sélection (S0-04)
- [ ] **S0-04a** (L, P0) Capture par presse-papiers : sauvegarde → simulation Ctrl+C → lecture → restauration.
- [ ] **S0-04b** (M, P0) Sauvegarde/restauration **multi-formats** (texte, RTF/HTML, images, fichiers) ; restauration garantie même en cas d'erreur.
- [ ] **S0-04c** (M, P0) Gestion des cas limites : aucune sélection, délai de copie asynchrone, application qui bloque la copie, interruption.
- [ ] **S0-04d** (M, P1) Piste de repli UI Automation / TextPattern : évaluer l'intérêt réel.
- [ ] **S0-04e** (M, P0) Panel de tests (navigateurs, Word, éditeur de code, PDF reader, terminal, app Electron, apps élevées/admin…) → **rapport de compatibilité** avec limites documentées.

### EP-04 Raccourcis globaux (S0-05)
- [ ] **S0-05a** (M, P0) Valider la bibliothèque de raccourcis globaux (plugin Tauri `global-shortcut` vs `RegisterHotKey`) ; détection hors focus.
- [ ] **S0-05b** (S, P1) Tester les raccourcis proposés (Ctrl+Alt+Espace / P / S) : conflits, échec d'enregistrement détecté et signalé.

### EP-05 Benchmarks (S0-07)
- [ ] **S0-07a** (M, P1) Protocole de benchmark reproductible (textes courts/moyens/longs, FR/EN).
- [ ] **S0-07b** (M, P1) Mesurer : latence avant 1re parole, temps de génération / durée audio (RTF), RAM, VRAM, charge CPU/GPU ; CPU vs GPU si dispo.
- [ ] **S0-07c** (S, P1) Tableau de résultats + verdict « mode CPU viable ? ».

### EP-06 Licences (S0-08)
- [ ] **S0-08a** (M, P0) Audit séparé : moteur, poids du modèle, voix, phonémiseur, ONNX Runtime, crates Rust, polices/assets.
- [ ] **S0-08b** (S, P0) Recommandations de redistribution (bundle vs téléchargement séparé, mentions requises).

### EP-07 Clôture du Sprint 0
- [ ] **S0-09a** (S, P0) Démo intégrée : raccourci → capture → TTS progressif → pause/stop.
- [ ] **S0-09b** (S, P0) Vérifier les 8 critères de réussite (doc §11) ; lister les incertitudes restantes.
- [ ] **S0-09c** (S, P0) Architecture validée/amendée + **backlog Sprint 1 priorisé** sur résultats réels (go/no-go).
- [ ] **S0-09d** (S, P1) Choix finaux consignés : voix FR/EN, libs audio et raccourcis.

---

## Sprint 1 (provisoire) — Tranche verticale minimale
*Si S0 validé. Sinon : traiter d'abord le risque majeur restant.*

### EP-10 Cœur applicatif
- [ ] **S1-01** (M, P0) Structure Rust modulaire : `capture`, `shortcuts`, `text`, `language`, `tts`, `audio`, `state`.
- [ ] **S1-02** (M, P0) Machine d'états de lecture (inactif / capture / génération / lecture / pause / erreur) avec événements vers l'UI.
- [ ] **S1-03** (M, P0) Instance unique ; démarrage résident sans fenêtre principale.
- [ ] **S1-04** (S, P1) Journalisation locale + gestion d'erreurs centralisée (aucune télémétrie distante).

### EP-11 Texte et langue
- [ ] **S1-05** (M, P0) Nettoyage/normalisation (espaces, sauts de ligne, URLs, symboles, nombres/abréviations de base).
- [ ] **S1-06** (M, P0) Détection automatique FR/EN indépendante du moteur TTS (+ comportement si langue inconnue / texte mixte).
- [ ] **S1-07** (M, P0) Segmentation en production + préparation de la file.
- [ ] **S1-08** (M, P1) Tests unitaires ciblés : nettoyage, segmentation, détection de langue.

### EP-12 Intégration TTS et audio
- [ ] **S1-09** (L, P0) Moteur Kokoro intégré derrière `TtsEngine` ; chargement du modèle en tâche de fond.
- [ ] **S1-10** (M, P0) Préchargement/anticipation des segments, réglage de la taille de segment selon les benchmarks.
- [ ] **S1-11** (M, P0) Lecture, pause, reprise, arrêt intégrés à la machine d'états.
- [ ] **S1-12** (S, P1) Gestion du périphérique audio par défaut (changement/absence de périphérique).

### EP-13 Capture et raccourcis en production
- [ ] **S1-13** (L, P0) Capture presse-papiers durcie (formats, restauration garantie, tests d'erreur/interruption).
- [ ] **S1-14** (M, P2) Repli UI Automation si retenu au S0.
- [ ] **S1-15** (M, P0) Raccourcis globaux (lire / pause-reprise / arrêt) ; échec d'enregistrement signalé à l'utilisateur.
- [ ] **S1-16** (M, P1) Messages clairs quand la capture échoue (aucune sélection, application non compatible).

### EP-14 Présence minimale
- [ ] **S1-17** (M, P0) Icône de zone de notification + menu (lire, pause/reprise, arrêt, paramètres, quitter).
- [ ] **S1-18** (L, P1) Panneau ancré à l’icône de notification (maquette A) : état, phrase en cours, segments, pause/reprise/arrêt, raccourcis. **Pas de mini-lecteur flottant** (pastille B écartée).
- [ ] **S1-19** (M, P1) Anneaux d’état de l’icône (maquette C) : prêt, génération (bleu tournant), lecture (cyan pulsant), pause (ambre), erreur (pastille rouge).
- [ ] **S1-20** (S, P1) Icône .ico multi-tailles à partir de « navi icon.jfif » ; valider la lisibilité 16-24 px.

---

## Epics suivants (Sprint 2+) — Reste de la vision

### EP-20 Panneau et UX
- [ ] (M, P2) Panneau abouti : ancrage, multi-écrans, DPI, barre en haut/côté.
- [ ] (M, P3) Accessibilité : navigation clavier, contrastes, lecteurs d'écran.

### EP-21 Paramètres
- [ ] (M, P1) Fenêtre de paramètres (React) ; persistance de la configuration locale.
- [ ] (M, P1) Raccourcis configurables + détection/gestion des conflits.
- [ ] (S, P2) Choix de la voix, vitesse de lecture, volume.
- [ ] (S, P2) Lancement au démarrage de Windows (activable/désactivable).

### EP-22 Signature sonore de démarrage
- [ ] (M, P3) Créer l'enregistrement **original** « Hey! Listen! »-inspiré (0,8-1,5 s, voix aiguë, volume modéré) — aucun extrait/imitation d'une franchise existante.
- [ ] (S, P3) Lecture une seule fois, uniquement quand raccourcis + capture sont opérationnels (sans suggérer que le TTS est prêt si le modèle charge encore).
- [ ] (S, P3) Réglages : désactivation et volume.

### EP-23 Modèles et distribution
- [ ] (M, P1) Gestion locale des modèles : emplacement, vérification d'intégrité, message si absents.
- [ ] (M, P2) Installeur Windows (MSIX/NSIS) ; décision bundle vs téléchargement selon l'audit de licences.
- [ ] (S, P1) Fichier de mentions légales/licences tierces.
- [ ] (M, P3) Installation/mise à jour automatique des modèles *(hors périmètre S0)*.

### EP-24 Qualité et robustesse
- [ ] (M, P1) Suite de tests : capture/restauration, file audio, machine d'états ; CI (build + tests + lint).
- [ ] (S, P2) Gestion mémoire : déchargement du modèle après inactivité (si RAM significative).
- [ ] (S, P2) Page « limites connues » : apps non supportées.
- [ ] (M, P3) Moteurs alternatifs (Piper/MeloTTS) via `TtsEngine` si besoin.

### Hors périmètre (explicitement exclus de la vision actuelle)
macOS/Linux · voix/profils/dictionnaires multiples · lecture de documents sans sélection · OCR · cloud/compte/télémétrie · UI définitive avancée.

---

## Points à confirmer (issus du doc §13)
- [ ] Voix FR/EN retenues · [ ] Qualité/latence/conso réelles de Kokoro · [ ] Libs Rust audio et raccourcis · [ ] Fiabilité presse-papiers vs UI Automation · [ ] Licences/redistribution · [ ] Forme finale de la signature sonore
