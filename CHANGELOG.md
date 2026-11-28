# Journal des versions

## Non publié

## 0.1.3 — 2026-11-28 (non signée)

### Ajouts

- Notification Windows quand la capture de la sélection échoue (message court et orienté action, par exemple « Cette application bloque la copie. Essayez de copier le texte manuellement. ») ou quand le modèle de lecture n'a pas pu être chargé. L'icône passe aussi au rouge.
- Journal local (`%APPDATA%\Navi\logs`) avec rotation à 2 Mo : erreurs, étapes et durées, sans aucune télémétrie. Le texte lu, dicté ou reconnu n'y est jamais écrit. Paramètres > Journaux pour ouvrir le dossier.

### Changements

- Fenêtre Paramètres : design harmonisé avec le panneau (thème sombre, mêmes couleurs, boutons et focus visible). Aucun changement fonctionnel.

### Développement

- Tous les diagnostics passent par la façade `log` (niveaux error/warn/info/debug) au lieu de `eprintln!`, perdu dans l'application installée ; `NAVI_LOG` règle le niveau en développement. Les erreurs de lecture et de dictée sont journalisées en un point unique. Crochet de panique : la cause est écrite avant l'arrêt (`panic = "abort"`).

## 0.1.2 — 2026-11-24 (non signée)

### Corrections

- Réinstaller le modèle de lecture ne ferme plus l'application (le moteur est conservé, seule sa session est rechargée).
- Espeak-ng introuvable ou sans données ne ferme plus l'application : erreur lisible à la place ; les données d'espeak-ng sont empaquetées sans aplatissement de dossiers.

### Dictée

- Le modèle de dictée est vérifié (SHA-256) puis chargé au lancement, en tâche de fond : la première dictée ne subit plus ~10 s d'attente.
- Cartouche au-dessus de la pastille : le texte reconnu s'affiche pendant l'enregistrement (retranscription toutes les 0,6 s), puis le texte final jusqu'à la saisie. Il s'élargit jusqu'à 200 px puis grandit vers le haut. Le texte n'est jamais journalisé.
- Pastille repositionnée (5 px au-dessus de la barre des tâches, décalée vers la gauche) et non activable.

### Développement

- Profil de dev : whisper.cpp et SHA-256 optimisés ; durées de la dictée journalisées.

## 0.1.1 — 2026-11-24 (non signée)

### Corrections

- Lecture d'une sélection : le moteur de lecture est maintenant créé (installation, vérification, chargement) au lieu d'échouer avec « Moteur TTS non chargé » quand la lecture précède la fin du chargement. L'erreur réelle est affichée en cas d'échec.

### Ajouts

- Paramètres > Modèles : bouton « Réinstaller le modèle de lecture » (suppression puis retéléchargement vérifié par SHA-256).

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
