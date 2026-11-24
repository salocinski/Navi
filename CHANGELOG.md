# Journal des versions

## 0.1.0 — 2026-11-23 (première version, non signée)

### Lecture à voix haute (Kokoro, local)

- Sélection capturée par presse-papiers (restauré ensuite), nettoyée, découpée et lue en continu ; français et anglais détectés automatiquement.
- Raccourcis globaux configurables : lire, pause/reprise, arrêt.
- Icône de zone de notification avec anneaux d'état, panneau ancré, pastille flottante, fenêtre Paramètres (langue, voix, vitesse, volume, signature sonore, démarrage avec Windows).

### Dictée vocale (Whisper Large v3 Turbo, local)

- Maintenir `Alt+Espace`, parler, relâcher : le texte est saisi dans la fenêtre active (pastille « REC » pendant l'écoute).
- Accélération GPU NVIDIA (CUDA 13, architectures 7.5, 8.6 et 8.9) avec repli processeur.
- Modèle (environ 1,6 Go) téléchargé une fois depuis Hugging Face, empreinte SHA-256 vérifiée ; section « Dictée vocale » des réglages (raccourci, micro, état du modèle).
- Aucun son conservé ni envoyé ; texte transcrit non journalisé.

### Système

- Navi s'exécute en administrateur (saisie dans les fenêtres élevées) ; démarrage à l'ouverture de session par tâche planifiée sans invite UAC ; installeur par machine (`Program Files`).
- Mentions légales des composants tiers dans `THIRD_PARTY_NOTICES.md`.

### Modèles

- Kokoro est installé au premier usage ; contrôle de mise à jour des modèles au démarrage.
- Mise à jour des modèles par l'utilisateur : manifeste signé (ed25519), remplacement atomique des fichiers et retour arrière en cas d'échec.
- Réouverture de la sortie audio quand le périphérique par défaut change.

### Limites connues

- **Installeur non signé** : Windows SmartScreen affichera un avertissement.
- Non vérifiés avant publication : tâche planifiée élevée réelle et démarrage à l'ouverture de session, saisie dans une fenêtre élevée, chaîne complète micro → Whisper sur l'installeur, machine sans GPU NVIDIA, lecteurs d'écran.
- Premier caractère parfois perdu dans un Bloc-notes activé sans clic (voir `docs/s3-96-compat-dictee.md`).
- Pas de reprise de téléchargement du modèle ; pas de CSP sur les pages internes.
