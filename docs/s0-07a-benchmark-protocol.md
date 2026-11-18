# S0-07a — Protocole de benchmark reproductible (carte TW-022)

Date : 2026-11-17. Statut : **défini** — l'exécution des mesures est la carte **TW-023**.
Référence : `docs/environnement-reference.md` (machine de mesure), `docs/s0-02.md` (chaîne Rust validée).

## Objectif

Définir un protocole de benchmark reproductible pour la chaîne TTS de Navi
(Kokoro-82M via ONNX Runtime, EP CPU) afin de pouvoir, dans les cartes
suivantes :

- **TW-023** : exécuter les mesures (latence, RTF, mémoire) ;
- **TW-024** : produire le tableau de résultats et statuer sur le mode CPU ;
- **TW-030** : prioriser le backlog Sprint 1 sur résultats réels ;
- **TW-028** : vérifier les critères de réussite du Sprint 0.

## Textes de référence (fixes, ne pas modifier sans PR)

Trois tailles, deux langues : **court** (1-2 phrases, ~20-30 tokens), **moyen**
(4-6 phrases, ~70-110 tokens), **long** (10-12 phrases, ~300-450 tokens).
Les comptes de tokens exacts seront relevés lors de la mesure (TW-023) et
consignés, pas devinés.

### FR

**Court FR**
"La synthèse vocale locale protège votre vie privée. Aucune donnée ne quitte la machine."

**Moyen FR**
"Navi transforme vos textes en voix de façon entièrement locale. Le moteur Kokoro lit les phrases les unes après les autres. Vous pouvez mettre en pause, reprendre ou arrêter à tout moment. Le logiciel ne contacte aucun serveur. Ouvrez une note, sélectionnez un paragraphe et écoutez-le."

**Long FR**
"Navi est un lecteur de texte vers la parole conçu pour Windows 11. Il fonctionne sans connexion réseau et sans compte utilisateur. Le modèle vocal Kokoro intègre quatre-vingt-deux millions de paramètres et tourne sur le processeur. Quand vous sélectionnez du texte dans une application, Navi le capture, nettoie les sauts de ligne et les abréviations, puis le segmente en phrases. La génération commence pendant que la première phrase est déjà en cours de lecture, ce qui donne une impression de continuité. En cas d'erreur, l'application affiche un message clair et réinitialise proprement son état interne. La mémoire utilisée par le modèle est libérée après une période d'inactivité prolongée. Les raccourcis clavier permettent de déclencher la lecture sans quitter votre travail. Ce protocole mesure les performances de cette chaîne sur la machine de référence."

### EN

**Short EN**
"Local text to speech keeps your data private. Nothing leaves this machine."

**Medium EN**
"Navi turns your text into speech entirely on your device. The Kokoro engine reads sentences one after another. You can pause, resume, or stop at any moment. The software never contacts a server. Open a document, select a paragraph, and have it read aloud."

**Long EN**
"Navi is a text to speech reader built for Windows 11. It works without a network connection and without an account. The Kokoro voice model has eighty-two million parameters and runs on the CPU. When you select text inside an application, Navi captures it, cleans up line breaks and abbreviations, then splits the selection into sentences. Generation starts while the first sentence is already playing, which gives the feeling of continuous reading. If something goes wrong, the application shows a clear message and resets its internal state gracefully. Memory used by the model is released after a long period of inactivity. Keyboard shortcuts let you trigger playback without leaving your work. This protocol measures the performance of the whole chain on the reference machine."

### Voix de référence

- FR : `fr_siwis` (déjà utilisée dans les échantillons), et `af_heart` comme voix non-native de contrôle si disponible.
- EN : `af_heart` (mesurée précédemment), `am_adam` comme seconde voix de contrôle si disponible.

> Les noms exacts des voix acceptées par `voices-v1.0.bin` seront confirmés au
> premier passage (TW-023) : si l'une n'existe pas, on la remplace par la voix
> FR/EN la plus proche, et on le consigne dans `docs/s0-07-report.md`.

## Environnement de mesure

**Machine de référence uniquement**, selon `docs/environnement-reference.md` :
MSI MS-7A71, Windows 10 Home 10.0.19045, i7-7700K (4C/8T), 32 Go (31,9 Go), RTX 3060 Ti 8 Go, driver NVIDIA 596.49, Rust 1.99.1, Node 26.4.0.
Étiqueter tous les résultats « mesuré sur Windows 10 Home ».

Conditions imposées à chaque run :

- Alimentation : plan **Performances élevées**, PC de bureau branché, pas sur secteur "économie".
- Journalisation et lancement : fermer navigateurs et apps lourdes (jeux, VM, IDE), pas de benchmark en arrière-plan.
- RAM disponible ≥ 8 Go libres avant chaque série (vérifier avec `Get-CimInstance Win32_OperatingSystem`).
- GPU dédié : le protocole est **CPU uniquement** ; le test GPU (TW-8 / S0-02c) est hors périmètre ici mais le protocole sera réutilisable tel quel.
- Modèle chargé depuis `kokoro-v1.0.int8.onnx` (int8), 24 kHz mono, vitesse `1.0`.
- Versions consignées dans le rapport : commit HEAD, `rustc --version`, `ort` 2.0.0-rc.15, checksum du modèle (déjà dans l'audit TW-25 — à reproduire dans le rapport).

## Métriques et unités

| Métrique | Définition | Unité |
|---|---|---|
| **TTFA** | Temps entre le lancement de la lecture d'un texte et l'émission du premier échantillon audio (hors démarrage/chargement modèle) | ms |
| **RTF** | temps_génération / durée_audio | sans unité (ex. 0,38) |
| **Temps de génération** | de la demande des tokens à la fin de l'inférence | s |
| **Durée audio** | durée totale de l'audio produit | s |
| **Chargement modèle** | coût unique de chargement ONNX (référence, déjà mesuré : 1,29 s) | s |
| **RAM pic** | crête mémoire physique du processus (Rust) pendant la génération | Mo |
| **VRAM** | crête VRAM si EP GPU utilisé — sinon **non applicable (CPU)** | Mo |
| **CPU moyen / pic** | charge totale processus (tous threads) pendant génération | % |
| **Fichiers produits** | WAV 24 kHz mono int16, nommés dans `scratch/benchmarks/` | — |

## Protocole d'exécution

1. **Préparation.** Récupérer le commit HEAD, les versions, l'état machine, afficher `nvidia-smi` (driver) et vérifier GPU dédié au repos.
2. **Chauffe.** Une génération FR-court et une EN-court **non comptabilisées** (allocation caches ONNX).
3. **Séries.** Pour chaque texte × langue × voix :
   - **3 runs CPU** (Rust, `--release`), mesure de **TTFA, RTF, temps gen, durée audio, RAM, CPU** ;
   - **3 runs Python de contrôle** (`kokoro-onnx`, déjà outillé), pour croiser avec `docs/s0-02.md` ;
   - valeurs retenues = **médiane** des 3 runs (+ min/max en note).
4. **Cold start.** Un run avec **modèle non chargé** pour mesurer le coût de chargement (référence : 1,29 s) et l'effet sur la première latence.
5. **Enregistrement.** CSV `scratch/benchmarks/AAAA-MM-JJ-taf/results.csv` avec une ligne par run : `commit, machine, OS, voix, langue, texte_id, taille, run_id, tttms, gen_s, audio_s, rtf, ram_mo, cpu_pct, cold_start`.
6. **Reproductibilité.** Les textes sont copiés tels que dans `scratch/benchmarks/texts/` (ou lus depuis ce document) ; le même script, la même machine, le même driver → mêmes chiffres à ±5 % près. Tout écart > 5 % impose une note dans le rapport (TW-024).

### Harnais

- Bac à sable existant `scratch/s0_02` (Rust, EP CPU) étendu avec une boucle sur les 6 textes. Fichiers **non versionnés** (cf. `.gitignore` `models/`), le protocole ne l'exige pas.
- Le moteur Python `kokoro-onnx` sert uniquement au contrôle ; le verdict CPU **ne se base pas** dessus.
- Les résultats partiels attendus, conformes à `docs/s0-02.md` : RTF CPU ~0,38 — ils seront re-mesurés proprement sur les 6 textes.

## Seuils go/no-go (proposés, à valider en TW-024)

| Verdict mode CPU | Condition |
|---|---|
| **CPU viable** | RTF ≤ 0,5 sur les 6 textes, TTFA ≤ 1,5 s sur court |
| **CPU limite** | 0,5 < RTF ≤ 0,8 sur au moins un texte — à surveiller |
| **CPU non viable** | RTF > 0,8 sur moyen/long, ou TTFA > 2,5 s — il faut alors le repli GPU (TW-8) ou Piper/MeloTTS (TW-67) |

Ces seuils sont des propositions : le verdict officiel sera consigné dans `docs/s0-07-report.md` (TW-024) **et** dans `decisions.md`.

## Critères d'acceptation (carte TW-022)

- [x] Textes court/moyen/long FR/EN fixés et joints au document
- [x] Métriques et unités définies (TTFA, RTF, RAM, VRAM, CPU)
- [x] Machine, OS, conditions et étiquetage définis
- [x] Protocole d'exécution reproductible (chauffe, 3 runs, médiane, CSV)
- [x] Voix de référence et points à confirmer en TW-023 listés
- [x] Seuils go/no-go proposés pour TW-024

La mesure elle-même est hors périmètre de cette carte (TW-023).
