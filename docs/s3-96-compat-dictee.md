# N-096 — Compatibilité de la saisie dictée : rapport

Date : 2026-11-23. Carte : N-096. Machine : poste du propriétaire (Windows 10 Home 19045), session **non élevée**.
Module testé : `insert::insert_text` (SendInput Unicode, le chemin de la dictée) via `cargo run --example insert_recipe`.
Texte témoin : `Dictée ✓ œuvre « test » 😀` + saut de ligne + `ligne 2` (accents, ligature, symbole, guillemets, emoji hors plan BMP, Entrée).

## Protocole

Chaque application est ouverte, mise au premier plan par script (le script refuse de saisir si une fenêtre du même exécutable n'a pas le premier plan), le curseur est placé dans la zone de saisie par un clic, puis l'exemple saisit le texte témoin. Contrôle : capture d'écran de la fenêtre, sauf PowerShell (texte relu dans un fichier écrit par `Read-Host`).

## Résultats (joués)

| Application | Verdict | Détail |
|---|---|---|
| Bloc-notes (Win10, classique) | ✅ (voir réserve) | Texte et saut de ligne exacts après un clic dans la zone de saisie |
| Google Chrome (`<textarea>`) | ✅ | Texte exact, emoji compris, saut de ligne conservé |
| Microsoft Edge (`<textarea>`) | ✅ | Idem |
| PowerShell (conhost, `Read-Host`) | ✅ | Première ligne relue à l'identique dans le fichier (accents, ✓, œ, « », emoji) ; l'affichage de la console remplace ✓ et l'emoji par des glyphes de repli, mais l'entrée est correcte |

### Réserve : premier caractère perdu sans clic

Dans un Bloc-notes tout juste ouvert, mis au premier plan par script **sans clic** dans la zone de saisie, la première saisie a perdu son premier caractère (« ictée … ») ; une seconde saisie dans la même fenêtre était complète, et la saisie est complète quand un clic précède. Cause non établie (initialisation du focus/de la saisie de la fenêtre après activation programmatique ?). En usage réel l'utilisateur est déjà dans le champ ; non reproduit hors banc d'essai, à surveiller lors de la recette réelle (N-092 à N-095 de bout en bout). Aucune contre-mesure codée.

## Non joué

- **Fenêtres élevées** (carte centrale de la dictée en administrateur) : non joué, la session de test n'est pas élevée et une élévation demande une invite UAC. Attendu : Navi élevé (N-095) peut saisir dans une fenêtre élevée ; une instance non élevée est bloquée par UIPI (`SendInput` renvoie 0, erreur lisible, N-091). **À jouer à la main** : lancer un Bloc-notes « Exécuter en tant qu'administrateur », puis `cargo run --example insert_recipe` depuis un terminal non élevé (attendu : échec) et depuis un terminal administrateur (attendu : succès).
- Word, VS Code, Discord, New Outlook : non installés ou non joués ; aucune présomption.
- Chaîne complète (raccourci maintenu, micro, Whisper, saisie) : non jouée, elle exige de parler ; à faire à la main sur un build release.
- Applications à `SendInput` filtré (jeux plein écran, certains clients distants/VM) : non testées.

## Rejouer

```
cargo run --example insert_recipe -- [--relecture] [secondes]
```

Compte à rebours, donner le focus à la cible, saisie. `--relecture` sélectionne tout (Ctrl+A) puis relit par la capture presse-papiers et compare (utile dans une zone de saisie déjà active).
