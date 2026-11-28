# S1-034 — Journalisation locale et centralisation des erreurs (carte N-034)

Date : 2026-11-27. Carte : N-034 (P2, S). Branche : `feat/N-034`.

## Ce que fait la carte

Navi écrit un **journal local** exploitable en cas de problème, **sans aucune télémétrie** : rien
n'est envoyé nulle part, aucun identifiant, aucune requête.

Avant la carte, tous les diagnostics passaient par `eprintln!` — une cinquantaine d'appels dont la
sortie est perdue dans l'application installée (`windows_subsystem = "windows"` : pas de console).
Les trois derniers bugs (N-099, N-102, N-103) ont été diagnostiqués par lecture du code, faute de
trace.

## Emplacement et rotation

| Élément | Valeur |
|---|---|
| Dossier | `%APPDATA%\Navi\logs` (`<données de l'application>/logs`) |
| Fichier courant | `navi.log` |
| Fichier précédent | `navi.log.1` |
| Taille maximale | 2 Mio par fichier, **un seul** historique conservé |
| Format | `2026-11-27T09:12:33.123Z INFO  navi_lib::capture: message` |

Le renommage se fait à la première écriture qui dépasserait la limite : le fichier est fermé
(verrou Windows), renommé en `.1`, puis recréé. Deux fichiers au plus, 4 Mio au total.

## Centralisation des erreurs

Deux points uniques journalisent tout ce qui atteint l'interface :

- `PlaybackStateMachine::set_error` (lecture) — `log::error!("lecture : …")` avant l'émission ;
- `Dictation::fail` (dictée) — `log::error!("dictée : …")`.

Les autres traces sont classées par niveau :

| Niveau | Usage | Exemples |
|---|---|---|
| `error` | l'action demandée échoue | chargement du moteur, synthèse, réinstallation du modèle, raccourci |
| `warn` | dégradation tolérée | config illisible, pastille ou tray indisponible, signature sonore absente, espeak-ng |
| `info` | cycle de vie | démarrage, lecture terminée, modèle déchargé, durées de dictée |
| `debug` | détail par segment ou par trame | synthèse segment *n*, file audio, progression du téléchargement |

## Confidentialité

Règle du module `diag` : **on ne journalise jamais le texte** lu, dicté, reconnu, ni un contenu de
presse-papiers. Seulement des longueurs, des langues, des identifiants de modèle et des messages
d'erreur. Deux garde-fous :

- les appels existants ne journalisent déjà que des compteurs (`{} caractères`, `{} échantillons`) ;
- chaque ligne est **tronquée à 2 000 caractères** : une journalisation accidentelle d'un texte
  reste bornée.

## Panique

Le binaire de livraison est compilé avec `panic = "abort"` : sans crochet, une panique ne laisse
aucune trace. `diag::init` installe un crochet qui journalise le message et l'emplacement
(`fichier:ligne:colonne`), force l'écriture sur disque, puis rend la main au crochet par défaut.

## Accès utilisateur

Paramètres → **Journaux** → « Ouvrir le dossier des journaux » (commande `open_log_dir`, ouverture
via le plugin `opener` déjà présent). Le dossier est créé au besoin.

## Développement

`NAVI_LOG=debug` (ou `trace`, `warn`, `error`, `off`) ajuste le niveau ; la variable est lue via
`utils::dev_env` et donc **ignorée en release**, comme les autres variables de développement. En
compilation de développement, chaque ligne est aussi recopiée sur `stderr`.

## Tests

Cinq tests unitaires dans `src-tauri/src/diag.rs` :

- horodatage epoch et date connue (`2023-11-14T22:13:20.123Z`, `2026-11-27T00:00:00.000Z`) ;
- troncature des lignes trop longues ;
- rotation : un seul fichier précédent, contenu du bon cycle ;
- dossier introuvable : le journal se désactive sans casser l'application.

Vérifications : `cargo fmt --check`, `cargo clippy --no-default-features --all-targets -D warnings`,
`cargo test --no-default-features` (182 tests verts, 7 ignorés).

## Limites assumées

- Pas de consultation intégrée : le journal s'ouvre dans l'éditeur ou l'explorateur.
- Pas d'envoi automatique ni de bouton « envoyer le journal » : l'utilisateur joint le fichier
  lui-même. C'est le prix de l'absence totale de télémétrie.
- Le niveau n'est pas réglable depuis l'interface ; `Info` par défaut suffit au diagnostic courant,
  `NAVI_LOG` reste un outil de développement.
- Les traces de la webview (console JavaScript) ne sont pas collectées : seuls les messages du
  processus Rust le sont.
