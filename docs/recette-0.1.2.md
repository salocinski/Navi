# Recette de la version 0.1.2 (carte N-110)

Date : 2026-11-26. Carte : N-110 (P1, M). Branche : `recette/N-110`.

## Objet

Fermer les réserves laissées ouvertes par deux cartes livrées, qui renvoyaient toutes deux à
l'installeur 0.1.2 :

| Carte | Réserve | Preuve attendue |
|---|---|---|
| N-102 | « Réinstaller le modèle de lecture » ne doit plus fermer l'application | essai manuel sur l'app installée |
| N-103 | espeak-ng empaqueté sans aplatissement, erreur lisible si les données manquent | contenu de l'installeur + essai manuel |

Version publiée : `v0.1.2` (2026-11-25), installeur `Navi_0.1.2_x64-setup.exe`,
SHA-256 `5d374d9666235870d1c1fc8da3bdd041d3d5bf586694ea04a6803a3eef82d69b`.

## Vérifications statiques (faites)

Reproductibles sur le dépôt, sans installer l'application.

| Contrôle | Commande | Résultat |
|---|---|---|
| Empreinte de l'installeur local = empreinte publiée | `Get-FileHash src-tauri/target/release/bundle/nsis/Navi_0.1.2_x64-setup.exe -Algorithm SHA256` | identique (`5d374d96…82d69b`) |
| Versions alignées | `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` | `0.1.2` partout |
| Version de l'installeur | propriétés du fichier | `FileVersion 0.1.2`, `ProductVersion 0.1.2` |
| L'installeur embarque le correctif N-102 | `navi.exe` daté du 2026-11-24 20:31, après la fusion de `46d219d` (14:05) et `f56b8d5` (14:43) | oui |
| espeak-ng non aplati | `installer.nsi` généré le 2026-11-24 20:27 : 364 fichiers sous `models\phonemizer\espeak-ng-data\`, dont `phontab`, `phonindex`, `en_dict`, `fr_dict` et `voices\` | arborescence préservée |
| Pas de résidu aplati dans l'installeur | `installer.nsi` : un seul fichier à la racine de `models\phonemizer\` (`espeak-ng.dll`) | aucun `*_dict` ni `phondata` à plat |
| Autres ressources | `models\kokoro\`, `resources\hey_listen.mp3`, `THIRD_PARTY_NOTICES.md`, `cublas64_13.dll`, `cublasLt64_13.dll` | présentes |
| Élévation | `src-tauri/navi-admin.manifest` : `requireAdministrator` | présent |

Remarque de build, sans effet sur la release : `src-tauri/target/release/models/phonemizer/`
contient encore 365 fichiers à plat (18,8 Mo, datés du 2026-11-16), restes du build 0.1.1 qui
avait aplati les données. L'installeur ne les embarque pas — `installer.nsi` prend ses fichiers
dans `models/phonemizer/` du dépôt, pas dans `target/release/`. Un `cargo clean` ou la
suppression du dossier avant un build évite la confusion.

## Recette manuelle (à faire par le propriétaire)

À dérouler sur l'application **installée**, pas en `npm run tauri dev`.

### A. Installation

- [ ] A1. Désinstaller la version précédente, puis installer `Navi_0.1.2_x64-setup.exe`
      (SmartScreen : « Informations complémentaires » → « Exécuter quand même » ; installeur non signé).
- [ ] A2. Vérifier l'arborescence installée : `%ProgramFiles%\Navi\models\phonemizer\espeak-ng.dll`
      **et** `%ProgramFiles%\Navi\models\phonemizer\espeak-ng-data\` (dossier, avec `phontab`,
      `phonindex`, `fr_dict`, `en_dict`).
- [ ] A3. Cas de mise à jour (facultatif mais utile) : installer 0.1.1 puis 0.1.2 par-dessus ;
      les anciens fichiers à plat restent dans `models\phonemizer\` — vérifier que la lecture
      fonctionne quand même (la recherche exige la DLL **et** `espeak-ng-data` dans le même dossier).

### B. Lecture (N-102 et N-103)

- [ ] B1. Sélectionner du texte dans Chrome, `Win+Alt+S` → la lecture démarre, en français puis en anglais.
- [ ] B2. Paramètres → « Réinstaller le modèle de lecture » → **l'application ne se ferme pas**,
      le modèle se retélécharge, puis une lecture suivante fonctionne (réserve N-102).
- [ ] B3. Données espeak-ng absentes : renommer `%ProgramFiles%\Navi\models\phonemizer\espeak-ng-data`
      en `espeak-ng-data.bak`, relancer Navi, tenter une lecture → message d'erreur lisible,
      **l'application reste ouverte** (réserve N-103). Restaurer le dossier ensuite.
- [ ] B4. Pause, reprise, arrêt (`Win+Alt+P`, arrêt) et enchaînement rapide de deux lectures.

### C. Dictée

- [ ] C1. `Alt+Espace` maintenu, parler, relâcher → le texte arrive dans la fenêtre active.
- [ ] C2. Première dictée après lancement : pas d'attente d'une dizaine de secondes
      (vérification SHA-256 et chargement en tâche de fond).
- [ ] C3. Le cartouche au-dessus de la pastille affiche le texte pendant l'enregistrement,
      puis le texte final.

### D. Interface et démarrage

- [ ] D1. Pastille à 5 px au-dessus de la barre des tâches, décalée vers la gauche, non cliquable,
      rouge en cas d'erreur.
- [ ] D2. Démarrage automatique : tâche planifiée, exécution en administrateur, instance unique
      (un second lancement n'ouvre pas de doublon).
- [ ] D3. Désinstalleur : les fichiers programme sont retirés ; noter ce qui reste
      (configuration, modèles téléchargés) pour la documentation.

## Critères de clôture de N-110

- Réserve N-102 fermée : B2 passe sur l'application installée.
- Réserve N-103 fermée : A2 conforme et B3 affiche une erreur sans fermer l'application.
- Aucune régression constatée en B1, B4, C1 à C3.

## Résultats

| Date | Section | Verdict | Note |
|---|---|---|---|
| | | | |
