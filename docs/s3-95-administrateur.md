# N-095 — Navi toujours en administrateur

Décision propriétaire : Navi s'exécute élevé, pour saisir le texte dicté dans les fenêtres élevées (UIPI bloque `SendInput` sinon, N-091).

## Ce qui change

- **Manifeste** `src-tauri/navi-admin.manifest` (`requireAdministrator`), embarqué par `build.rs` **en release uniquement** : `tauri dev` et `cargo run` restent non élevés (sinon erreur 740 hors terminal administrateur). Un `tauri build` produit l'exécutable élevé ; `tauri build --debug` ne l'est pas.
- **Démarrage automatique** : une valeur `Run` ne lance pas une application élevée. `autostart.rs` crée une tâche planifiée `Navi` (ouverture de session, `HighestAvailable`, sans invite UAC, répertoire de travail = dossier de l'application). L'ancienne valeur `Run` (N-054) est migrée au lancement puis retirée.
- **Installation** : NSIS en `perMachine` (`Program Files`). En release, l'activation du démarrage est refusée si l'exécutable n'est pas dans `Program Files`.

## Durcissement (un processus élevé fait confiance à ce que l'utilisateur peut écrire)

- Un processus non élevé du même utilisateur peut modifier ses variables d'environnement, `%APPDATA%\Navi\config.json` et tout dossier utilisateur. Conséquences traitées :
  - Les variables `NAVI_MODELS_DIR`, `NAVI_SIGNATURE_*`, `NAVI_STT_MODEL` ne sont lues qu'en debug (`utils::dev_env`).
  - `espeak-ng.dll` ne se charge, en release, que depuis le dossier de l'application (`utils::may_load_code_from`) ; sinon repli sur l'installation système d'eSpeak NG.
  - Le fichier XML de la tâche est créé dans `C:\Windows\Temp` (`create_new`), `schtasks.exe` est appelé par chemin absolu.
  - `models_dir` doit être un chemin absolu.
- Interface web : la permission `opener:default` est retirée des capacités (l'interface n'ouvre ni lien ni dossier). Les commandes restantes valident leurs entrées (`set_config`).

## Limites assumées

- `shortcut:` manifeste élevé en release seulement (voir plus haut).
- Pas de CSP (`csp: null`) : les pages sont locales et sans contenu distant ; poser une CSP exige un essai visuel de chaque fenêtre, non fait ici.
- Un utilisateur standard qui s'élève avec les identifiants d'un autre compte administrateur exécute Navi avec le profil de cet autre compte (configuration différente) ; non géré.
- Si une instance non élevée tourne (build debug), l'instance élevée ne peut pas lui parler (UIPI) : fermer l'autre instance.

## Vérifications

- Tests : XML de tâche (échappement, UTF-16), détection de `Program Files`, validation de `models_dir`, cycle réel création/lecture/suppression d'une tâche **sans privilèges** (test ignoré `tache_reelle_…`).
- Non vérifié : création de la tâche **élevée**, invite UAC, démarrage réel à l'ouverture de session, installeur `perMachine`, saisie dans une fenêtre élevée.
