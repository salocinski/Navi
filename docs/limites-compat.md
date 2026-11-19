# Limites connues de compatibilité — capture presse-papiers et lecture

Date : 2026-11-19. Carte : N-064 (P2, S). Sources : `docs/s1-16-compat.md` (rapport partiel, clôture avec limitation tracée), `docs/s1-43-capture-restauration.md`, `docs/s1-15-ui-automation.md`, `docs/s1-14-cas-limites-capture.md`, `src-tauri/src/capture/mod.rs`.

> Ce document consolide **uniquement ce qui a été mesuré**. « Non mesuré » ≠ « ne marche pas » :
> c'est une ignorance tracée, pas un verdict. Toute ligne de ce fichier doit pouvoir pointer
> vers une recette ou une sonde jouée.

## Ce qui est validé (recettes jouées, 2026-11-18)

| Cas | Résultat | Source |
|---|---|---|
| Word — texte riche multi-formats (CF_UNICODETEXT + CF_HTML) | Capture + restauration intégrale du témoin | s1-43 |
| Chromium / Electron — sélection web | Capture + restauration du presse-papiers témoin | s1-43, s1-15 |
| Segmentation → TTS Kokoro | Chaîne validée (S1) | s1-37, s1-39… |

Machine de référence : Windows 11 (Win 10 constaté par endroits selon docs Sprint 0).

## Applications et contrôles non pris en charge

| Application / contrôle | Statut | Pourquoi |
|---|---|---|
| **Apps élevées / administrateur** (PowerShell élevé, etc.) | **Non pris en charge** (SendInput bloqué par UIPI) | Un processus non élevé ne peut pas envoyer Ctrl+C à une fenêtre en niveau d'intégrité supérieur. Nuance : la capture **détecte** l'échec via le numéro de séquence du presse-papiers (qui ne change pas) et le signale — ce n'est pas une copie silencieuse de contenu périmé. Non confirmé par mesure, attendu d'après l'API. |
| **Canvas / surfaces dessinées** (jeux, dessin, contrôles custom) | Non pris en charge | Aucun presse-papiers standard peuplé par Ctrl+C sur ces surfaces. |
| **Contrôles XAML modernes (Notepad actuel)** | Inconnu — voie UIA absente | Deux sondes UIA (2026-11-18) : `TextPattern` unsupported, arbre vide, `ValuePattern` unsupported. La voie SendInput n'a jamais été mesurée sur cette app. |
| **Outlook MSIX / WebView2 (`olk.exe`)** | Inconnu | WebView2 ≠ Chromium embarqué à l'identique ; aucune mesure. |
| **Lecteur PDF (Edge intégré ou dédié)** | Inconnu | Non testé (Edge PDF) ; aucun lecteur PDF dédié installé sur la machine de référence. |
| **Terminaux (conhost, Windows Terminal)** | Non testés | Console : Ctrl+C avec sélection = copie (comportement documenté, attendu), non vérifié ; Windows Terminal absent. |
| **GDI handles (CF_BITMAP, etc.)** | **Jamais restaurés** | API `windows` 0.58 : handles non copiables ; la restauration multi-formats est partielle par construction (voir s1-43, « Limite connue »). |
| **Apps gelées / hang** | Non testées | SendInput peut être ignoré ; l'expiration par séquence le détecterait, mais jamais joué. |
| **Presse-papiers verrouillé par une app tierce** | Non testé | `OpenClipboard` en échec → erreur remontée par la capture (chemin codé et testé en unitaire), pas joué sur machine. |
| **Presse-papiers re-écrit par l'utilisateur pendant la capture** (~1,5 s) | Comportement connu, accepté | La restauration écrase le nouveau contenu (documenté, s1-43). |

## Repli UI Automation : abandonné (NO-GO)

Évalué N-015 (2026-11-18) : **NO-GO** — pas de TextPattern sur Notepad moderne (2 sondes), UIPI bloque le cas exact où il servirait (apps élevées), coût d'intégration COM élevé pour un résultat partiel. N-044 (repli UIA) doit être retirée/reportée, pas implémentée.

## Raccourcis globaux (rappel)

Enregistrement **au niveau OS** : les raccourcis par défaut (Ctrl+Maj+Espace / Ctrl+Maj+P / Ctrl+Maj+S) se déclenchent même quand l'app ciblée est élevée — l'OS livre l'événement à Navi, hors UIPI. C'est **la capture de texte** qui échoue dans ce cas, pas le raccourci. Conflit mesuré : Ctrl+Alt+Espace réservé (voir N-018) ; défaut retenu : Ctrl+Maj+Espace.

## Ce qui n'est pas mesuré du tout

- Chrome et Edge en test **individuel** (tous deux couverts par la classe Chromium, pas mesurés séparément),
- VS Code et Discord en test individuel (couverts par la classe Electron),
- Edge PDF, Notepad XAML, conhost, élevée, Outlook MSIX,
- sélection image (CF_DIB) / fichier (CF_HDROP),
- restauration quand le presse-papiers est verrouillé,
- Word récent-non-testé après évolution (une seule version validée).

## Comment faire progresser ce document

Chaque entrée « Inconnu / non testé » se lève en rejouant la recette `cargo run --example capture_recipe`
(docs/s1-43-capture-restauration.md) et en reportant ici le verdict — pas en présumant.
Un contrôle reste « non pris en charge » tant que personne ne l'a vu passer.

## Traçabilité

Carte N-064. Sources : `docs/s1-16-compat.md` (dont correction Word, 2026-11-19), `docs/s1-43-capture-restauration.md` (validation terrain 2026-11-18), `docs/s1-15-ui-automation.md` (NO-GO UIA), `docs/s1-14-cas-limites-capture.md`, `src-tauri/src/capture/mod.rs` (séquence presse-papiers, tests `sequence_*`).
