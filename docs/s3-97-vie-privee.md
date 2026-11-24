# N-097 — Vie privée et accessibilité de la dictée

## Vie privée (audité dans le code, 2026-11-23)

- **Aucun audio sur disque** : `MicCapture` accumule des échantillons en mémoire (60 s maximum) ; `process` les passe à Whisper puis les libère en fin de fonction. Aucune écriture de WAV hors du test ignoré de l'étape N-090 (fichier fourni par l'utilisateur, jamais produit par l'application).
- **Aucun envoi** : le seul accès réseau lié à la dictée est le téléchargement du modèle (`stt/model.rs`, Hugging Face, une seule fois, sans audio ni identifiant). Aucun envoi de son, de texte ou de statistique.
- **Texte non journalisé** : le journal ne contient que la fin d'appui et sa durée (`dictée : … après N ms`), jamais le texte transcrit. Le texte part par `SendInput` : le presse-papiers n'est pas utilisé.
- **Indicateur micro** : la pastille rouge « REC » est affichée pendant toute l'écoute (N-092), en plus de l'indicateur de micro de Windows. L'écoute ne dure que tant que le raccourci est maintenu.

## Accessibilité

- Les icônes de la pastille portent un `aria-label` ; une zone cachée `role="status"` annonce désormais les changements de phase (enregistrement, transcription, saisie, erreur) aux lecteurs d'écran.
- Animations coupées si `prefers-reduced-motion`. L'état n'est pas porté par la couleur seule (rouge + point/forme d'enregistrement + texte annoncé).
- Réglages de dictée : champs avec libellés, raccourci saisissable au clavier.

## Limites

- La pastille ne prend pas le focus (volontaire : elle ne doit pas voler la saisie) ; l'annonce dépend donc de la prise en charge des régions vivantes par le lecteur d'écran. Non testé avec NVDA/Narrateur.
- La mémoire libérée n'est pas écrasée explicitement (`shortcut:` pas de remise à zéro des échantillons ; un processus élevé ne se laisse lire que par un autre processus élevé).
- Pas de contrôle automatisé empêchant un futur accès réseau ; garde par revue de code.
