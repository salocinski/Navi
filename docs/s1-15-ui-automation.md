# S1-015 — UI Automation / TextPattern en repli de capture : évaluation (carte N-015)

Date : 2026-11-18. Carte : N-015 (P2, M). Verdict : **NO-GO pour le Sprint 1** (et probablement pour la capture générale).
Sources : sonde PowerShell locale (UIAutomationClient .NET, machine de référence), notes d'API `windows` crate 0.58, résultats Sprint 1 (Ctrl+C → `SendInput` validé par l'humain sur Chromium et Word).

## Question de la carte

L'intérêt réel d'un repli via UI Automation / TextPattern (`IUIAutomation`) pour lire la sélection, là où la capture Ctrl+C peut échouer.

## Sondages mesurés (2026-11-18, Windows 10 Home, Notepad moderne)

| Sonde | Résultat |
|---|---|
| Arbre UIA → `FindFirst(Edit)` sous la racine Notepad | **Aucun `Edit` trouvé** (l'éditeur virtualisé n'expose pas un Edit dans l'arbre standard) |
| `AutomationElement.FocusedElement` avec focus + Ctrl+A | `ControlType.Edit`, `Name = « The editor is not accessible at this time. To enable screen reader optimized mode, use Shift+Alt+F1 »` |
| `TextPattern.DocumentRange.GetText(-1)` | **Retourne le libellé placeholder**, pas le texte du document |
| `TextPattern.GetSelection().GetText(-1)` | `selection-count = 1` mais `GetText` = **chaîne vide** |
| `ValuePattern.Current.Value` | vide |

Conclusion locale : le nouveau Notepad n'expose **aucune** API UI Automation texte en mode standard — le texte réel n'apparaît qu'avec l'éditeur en « screen reader optimized mode » (Shift+Alt+F1), ce qui exigerait de **basculer l'accessibilité de l'utilisateur** ou d'exiger ce mode. Inacceptable pour un repli produit.

## Contraintes structurelles (API)

1. **Cross-integrity level** : UIA est soumise à la barrière d'intégrité (UIPI). Un Navi non élevé ne peut pas automatiser une app élevée/admin — **le cas même où l'on veut un repli est exactement celui où UIA est indisponible**.
2. **Support des contrôles** : TextPattern est implémenté par les champs de texte standard (WinForms/WPF/Notepad via RichEdit, Word), jamais de façon uniforme par Chromium/Electron (fuit via `ILegacyIAccessible`/range par nœud, partiel), absent sur canvas/terminal/jeux.
3. **Coût d'ingénierie via `windows` crate 0.58** : `IUIAutomation` + pattern requis en COM vtable unsafe, découverte de l'élément focus, gestion des vtræds UI — facilement 2 à 4 jours pour un résultat partiel, plus le risque de régressions sur la capture qui marche déjà.
4. **Alternative sans dépendance** : `ITextProvider`/`TextPattern` en .NET (PowerShell/UIAClient) rend le diagnostic trivial — c'est ce qui a produit les sondes ci-dessus.

## Ce que Ctrl+C/SendInput couvre déjà (Sprint 1)

| App | Résultat |
|---|---|
| Chromium / Electron | ✅ capture + restauration OK (recette humaine) |
| Word (texte riche multi-formats) | ✅ capture + restauration OK |
| PDF / terminal / jeux/canvas | non mesurés — aucun signal TextPattern ne serait mieux, et Word resterait couvert par les deux voies |

## Verdict

**NO-GO.** Le repli UI Automation :
- n'apporte rien pour les deux apps critiques déjà couvertes ;
- échoue sur le cas Notepad moderne (le plus simple des cas texte) ;
- est bloqué par UIPI sur les apps élevées — précisément les cas « Ctrl+C ne marche pas » ;
- coûte plus cher (≥ 2 j) que la valeur attendue.

**Reco associée** : fermer **N-044** (repli UI Automation) en la sortant du Sprint 1 — documenter l'abandon ici plutôt que la réaliser. Réévaluer uniquement si **N-016** (rapport de compatibilité empirique) révèle des apps où Ctrl+C échoue **et** UIA lit effectivement — aucune des sondes ci-dessus n'en donne le moindre exemple.

Si un repli reste nécessaire plus tard, la piste la moins chère est **WinRT `ISelectionProvider` / `UIAutomationTextRange` ciblé** (cf. `docs/decisions.md`, entrée TextProviders) pour les champs standard, pas UIA générique.

## Limites

- Word non sondé par UIA dans cette évaluation (Word est réputé exposé ; mais Ctrl+C y fonctionne, donc nul enjeu).
- Chromium non sondé en live (aucun profil garantissant un contenu sélectionnable au moment du test ; support connu pour champs éditables uniquement).
- Conclusion repose sur un échantillon n=1 par sonde (Notepad) + connaissances API — suffisant pour un go/no-go de priorité P2.
