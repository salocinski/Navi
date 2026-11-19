# TW-072 — Raccourci « une main » pour lire à voix (étude de conflits)

Date : 2026-11-19. Carte : N-072 (P1, S, decision). Outil de mesure : `cargo run --example hotkey_check` (Machine de référence MSI MS-7A71, Windows 10 Home).

## Question de la carte

Trouver une combinaison **une main**, facile, qui ne casse pas les apps web (Google Docs, Chrome/Edge devtools, etc.).

## Observations sur la machine de référence

Mesure faite avec `hotkey_check` (chaque combinaison enregistrée, process court, puis libérée à la sortie) :

| Combinaison | Résultat machine | Commentaire |
|---|---|---|
| Ctrl+Maj+Espace (défaut actuel) | ECHEC | déjà réservée par l'OS/une app |
| Ctrl+Alt+Espace | ECHEC | déjà réservée |
| Win+Ctrl+Espace | ECHEC | déjà réservée |
| **Win+Alt+Espace** | **OK** | disponible ici ; 1 seule main, touche Win+Alt+Espace |
| Ctrl+Alt+E | OK | 1 combinaison simple |
| Ctrl+Alt+A | OK | |
| Ctrl+Alt+K | OK | |
| Ctrl+Alt+M | OK | |
| Ctrl+Alt+L | OK | |
| Ctrl+Alt+S | OK *(une seule fois par process)* | disponible |
| Ctrl+Alt+P | OK | |
| Alt+Shift+Espace | OK | mais **Alt+Shift** change la langue du clavier Windows |
| Win+Alt+Espace | OK final | **recommandé** |

Résultat machines tests seulement ; les conflits peuvent varier sur d'autres machines — la détection se fait à l'enregistrement (TW-018) et l'humain doit signaler la sienne.

## Pourquoi les conflits web actuels posent problème

| App | Combinaison en conflit | Comportement |
|---|---|---|
| Chrome / Edge | Ctrl+Maj+Espace | ouvrir la console / devtools en mode raccourci |
| Google Docs | Ctrl+ Maj+Espace | boîte de dialogue « nombre de mots » / actions clavier UI |
| Word / Outlook | — | pas d'interférence notable |
| Windows 10 | Ctrl+Maj+Espace | souvent réservée à l'OS ou un outil sur ce poste |

## Recommandation

**`Win+Alt+Espace`** : combinaison une-main-gauche, sur *Espace* (large touche naturelle au pouce), très peu utilisée par les apps (aucune UI commune n'exploite Win+Alt), libre sur la machine de référence, pas de conflit Windows connu (Win+Alt n'est pas un préfixe d'accessibilité OS standard).

Le processus retentera l'enregistrement sur la machine de l'humain et affichera `hotkey_error` si pris.

**Alternatifs acceptables si Win+Alt+Espace est pris chez l'humain** :
1. `Ctrl+Alt+Espace` → ECHEC chez nous, mais OK ailleurs si le leur.
2. `Ctrl+Alt+K` → lisible, libre ici.
3. `Ctrl+Alt+A` → très court, proche `Ctrl+A`.

## Préconisation

**Garder par défaut `Win+Alt+Espace`** ; l'humain peut surchager via settings (TW-052, raccourcis configurables).

**Action immédiate** : demander à l'humain quelle est la sienne ; s'il n'est pas d'accord, basculer sur Option B (Ctrl+Alt+K).

## Traçabilité

- Méthode : `cargo run --example hotkey_check`.
- Décision associée : TW-018 (Ctrl+Maj+… échec) et TW-045 (Câblage effectif).
