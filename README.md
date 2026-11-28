# Navi

Assistant vocal **local** pour Windows : Navi lit à voix haute le texte sélectionné et transcrit la voix en texte, dans n'importe quelle application. Après l'installation des modèles, tout fonctionne hors ligne : aucun son et aucun texte ne quittent la machine.

> Version actuelle : **0.1.3** (installeur non signé, Windows SmartScreen affichera un avertissement). Voir le [journal des versions](CHANGELOG.md).

## Fonctionnalités

### Lecture à voix haute (Kokoro)

- Sélectionnez du texte dans n'importe quelle application, puis appuyez sur **Win+Alt+S** : la sélection est capturée (via le presse-papiers, restauré ensuite), nettoyée, découpée et lue en continu.
- Français et anglais détectés automatiquement ; voix et vitesse réglables.
- **Ctrl+Maj+P** : pause et reprise. **Ctrl+Maj+S** : arrêt.
- Moteur Kokoro (ONNX Runtime), téléchargé une fois au premier usage et vérifié par SHA-256. Bouton « Réinstaller le modèle de lecture » dans les réglages.

### Dictée vocale (Whisper)

- Maintenez **Alt+Espace**, parlez, relâchez : le texte est saisi dans la fenêtre active.
- Whisper Large v3 Turbo, accéléré par GPU NVIDIA (CUDA 13), avec repli sur le processeur.
- Un **cartouche** au-dessus de la pastille affiche le texte reconnu pendant que vous parlez, jusqu'à la saisie.
- Le modèle (environ 1,6 Go) est téléchargé une fois depuis Hugging Face, vérifié (SHA-256) puis chargé au lancement, en tâche de fond : la première dictée est immédiate.
- Aucun son conservé, texte transcrit non journalisé.

### Interface

- Icône de zone de notification avec anneaux d'état (prêt, génération, lecture, pause, erreur) et panneau ancré.
- Pastille flottante animée au-dessus de la barre des tâches, non cliquable et sans prise de focus.
- Fenêtre Paramètres : langue, voix, vitesse, volume, raccourcis, microphone, signature sonore au démarrage, démarrage avec Windows.

### Système

- S'exécute en **administrateur** pour pouvoir saisir du texte dans les fenêtres élevées ; démarrage à l'ouverture de session par tâche planifiée, sans invite UAC.
- Installeur par machine (`Program Files`), en NSIS.
- Réouverture automatique de la sortie audio quand le périphérique par défaut change.

### Modèles

- Mises à jour des modèles par l'utilisateur via un **manifeste signé** (ed25519), remplacement atomique des fichiers et retour arrière en cas d'échec.
- Contrôle hebdomadaire de disponibilité (optionnel).

## Installation

1. Téléchargez `Navi_<version>_x64-setup.exe` depuis les [releases](https://github.com/salocinski/Navi/releases).
2. Lancez l'installeur (invite UAC), puis Navi. Au premier usage, les modèles sont téléchargés (lecture : environ 115 Mo ; dictée : environ 1,6 Go).

Prérequis : Windows 10 ou 11 x64. Une carte NVIDIA est recommandée pour la dictée (le processeur seul est très lent).

En cas de problème, le journal local est dans `%APPDATA%\Navi\logs` (Paramètres → Journaux → « Ouvrir le dossier des journaux »). Il ne quitte jamais l'ordinateur et ne contient aucun texte lu ou dicté.

## Développement

Pile : [Tauri 2](https://tauri.app/) (Rust) + React / TypeScript, `ort` (Kokoro), `whisper-rs` (Whisper).

Prérequis (Windows) : Rust stable, Node.js, Visual Studio avec la charge « Développement Desktop en C++ » et le SDK Windows, CUDA 13.5 avec son intégration Visual Studio, CMake. Les variables à définir pour compiler `whisper-rs-sys` (`CUDA_PATH`, `BINDGEN_EXTRA_CLANG_ARGS`, `CMAKE_CUDA_ARCHITECTURES`) sont décrites dans [docs/release.md](docs/release.md) et [docs/environnement-reference.md](docs/environnement-reference.md). Si Visual Studio n'est pas dans son emplacement par défaut, indiquer son dossier à CMake avec `CMAKE_GENERATOR_INSTANCE` (sinon `whisper-rs-sys` échoue sur « could not find specified instance of Visual Studio »).

```bash
npm ci
npm run tauri dev       # instance de développement (non élevée)
npm run lint            # ESLint
npx prettier --check .  # format
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --no-default-features
```

- Construire l'installeur : voir [docs/release.md](docs/release.md) (`npm run bundle:win`).
- Le dépôt est suivi avec ATLAS : conventions de travail et cycle des cartes dans [AGENTS.md](AGENTS.md).

## Licence

Navi est distribué sous licence **GPL-3.0** (voir [LICENSE](LICENSE)). Les composants tiers (Kokoro, Whisper, ONNX Runtime, espeak-ng, CUDA…) sont listés dans [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
