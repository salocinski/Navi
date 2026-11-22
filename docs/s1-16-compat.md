# S1-016 — Panel d'applications : rapport de compatibilité (carte N-016)

Date : 2026-11-19 (mise à jour). Carte : N-016 (P1, M). Machine : PC de l'humain (Windows 11 / Win 10 constaté selon docs antérieures).
Module testé : `src-tauri/src/capture/mod.rs` via l'exemple `cargo run --example capture_recipe`
(snapshot presse-papiers → SendInput Ctrl+C → lecture → restauration systématique, voir `docs/s1-43-capture-restauration.md`).

> **Limitation actée (2026-11-19)** — clôture de la carte à la demande de l'humain **avec limitation tracée**.
> Le panel complet n'a pas été joué : ce rapport repose sur les recettes humaines déjà jouées
> (N-043, 2026-11-18 : Word texte riche ; Chromium/Electron) et sur les sondes N-015 (2026-11-18, Notepad).
> Les applications marquées « non joué » n'ont fait l'objet d'**aucune** mesure ; aucune n'est présumée fonctionner.
> Le critère de la carte (« Rapport avec applications et contrôles non pris en charge ») est rempli
> **partiellement** : voir § « Applications et contrôles non pris en charge ».

## Protocole

Pour chaque application :

1. Préparer un **témoin** : copie d'un texte riche dans une autre app (vérifier qu'il est collable) ;
2. Ouvrir l'app cible, **sélectionner du texte sans le copier** ;
3. Lancer `cargo run --example capture_recipe`, donner le focus à l'app cible pendant le compte à rebours ;
4. Lire le rapport (texte capturé, format, restauration) ;
5. Coller ailleurs : le témoin doit revenir intact.

Verdicts : ✅ capturé + restauré · ⚠ partiel (texte capturé, restauration partielle) · ❌ échec (aucune capture).

## Panel — état au 2026-11-19

| # | Application | Catégorie | Verdict | Notes |
|---|-------------|-----------|---------|-------|
| 1 | Google Chrome (navigation web) | Navigateur (Chromium) | ✅ via classe Chromium | Non joué individuellement ; recette N-043 2026-11-18 : capture + restauration OK (Chromium/Electron) |
| 2 | Microsoft Edge (page web) | Navigateur (Chromium) | ✅ via classe Chromium | Non joué individuellement ; même moteur Chromium que la recette N-043 |
| 3 | Microsoft Edge (PDF intégré) | Lecteur PDF | Non joué | `navi-test.pdf` généré pour ce test ; jamais utilisé |
| 4 | Visual Studio Code | Éditeur de code (Electron) | ✅ via classe Electron (présumé) | Non joué individuellement ; Electron couvert par la recette N-043 (2026-11-18) |
| 5 | Discord | App Electron | ✅ via classe Electron (présumé) | Non joué individuellement ; Electron couvert par la recette N-043 (2026-11-18) |
| 6 | New Outlook (`olk.exe`, MSIX/WebView2) | App de messagerie (MSIX) | Non joué | WebView2 ≠ Chromium embarqué identique ; aucune mesure — **ne pas présumer** |
| 7 | PowerShell (conhost) | Terminal | Non joué | Ctrl+C avec sélection = copie sous conhost (comportement console documenté) ; non vérifié |
| 8 | Bloc-notes (Notepad, XAML) | Éditeur simple | Non joué | Sondes UIA N-015 (×2) : aucun TextPattern ; la voie SendInput n'a pas été mesurée sur cette app |
| 9 | PowerShell en mode élevé (administrateur) | App élevée | Non joué | SendInput bloqué par UIPI **attendu** ; non confirmé par mesure |
| — | Microsoft Word | Traitement de texte | ✅ | Recette N-043 (2026-11-18) : texte riche multi-formats (CF_UNICODETEXT + CF_HTML + formats internes), capture + témoin restauré intégralement. Annote : l'ancienne mention « non installé » de ce rapport était erronée et est corrigée ici |
| — | Adobe Acrobat / SumatraPDF | Lecteur PDF dédié | Non testé : non installé | Remplacé par le lecteur PDF d'Edge (lui-même non joué) |
| — | Windows Terminal | Terminal moderne | Non testé : non installé | Remplacé par PowerShell conhost (lui-même non joué) |

## Résultats détaillés

### Résultats issus des recettes jouées (preuves)

- **Word (texte riche)** — docs/s1-43-capture-restauration.md, validation terrain 2026-11-18 :
  sélection ailleurs → capture → Ctrl+V : réussite ; presse-papiers témoin restauré intégralement
  (cas le plus exigeant : multi-formats).
- **Chromium / Electron** — docs/s1-43-capture-restauration.md et docs/s1-15-ui-automation.md,
  2026-11-18 : sélection web → capture : texte capturé, presse-papiers restauré.

### Non joués (aucune mesure)

Edge PDF intégré, New Outlook (MSIX/WebView2), PowerShell conhost, Notepad XAML, PowerShell élevé,
Chrome et Edge en test individuel, VS Code et Discord en test individuel.

## Applications et contrôles non pris en charge

Sur la base des éléments **mesurés** (pas d'extrapolation) :

1. **Apps élevées / administrateur** — SendInput est bloqué par UIPI pour un processus non élevé ;
   la capture y est attendue en échec. Non confirmé par mesure (non joué) ; à traiter comme non pris en charge.
2. **Contrôles sans presse-papiers standard** (canvas, jeux, surfaces dessinées, terminaux en mode
   sélection interne) — aucune voie Ctrl+C garantie ; non mesurés, non pris en charge à ce stade.
3. **Formats GDI (CF_BITMAP et handles)** — jamais restaurés par la capture (limite API `windows` 0.58,
   N-043) : la restauration multi-formats est partielle par construction pour ces formats.
4. **Presse-papiers re-écrit pendant la capture** (~1,5 s) — la restauration écrase le contenu
   volontairement recopié par l'utilisateur (comportement connu et accepté, N-043).
5. **Notepad moderne (XAML)** — TextPattern absent (N-015, deux sondes) ; le repli UIA est NO-GO ;
   seule la voie SendInput reste possible, non mesurée.
6. **Outlook MSIX / WebView2** — statut inconnu ; aucune voie validée.

Non couverts par ce rapport (à jouer si la carte est rouverte) : Edge PDF, conhost, Notepad, élevé,
Outlook, et les tests individuels Chrome/Edge/VS Code/Discord.

## Traçabilité

- Consolidé dans `docs/limites-compat.md` (carte N-064) — la version de référence des limites.
- Carte : N-016. Sources : `src-tauri/examples/capture_recipe.rs`, `docs/s1-43-capture-restauration.md`,
  `docs/s1-15-ui-automation.md`, sondes et recettes humaines du 2026-11-18.
- **Limitation tracée (2026-11-19)** : clôture demandée par l'humain avec rapport partiel.
  Les verdicts ✅ sont adossés à des recettes jouées ; tous les autres cas sont explicitement
  marqués « non joué » ou « non installé ». Les résultats ne présument rien au-delà des mesures.
- `navi-test.pdf` (racine, non commité) : fichier de test généré pour Edge PDF, resté inutilisé.
  Supprimé le 2026-11-22 avec les autres éléments temporaires du dossier de travail (carte N-079,
  script `clean-workdir.ps1`) ; à régénérer si le cas Edge PDF est rouvert.
