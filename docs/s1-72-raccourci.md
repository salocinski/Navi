# TW-072 — Raccourci « une main » pour lire à voix (étude de conflits)

Date : 2026-11-19 (mise à jour 2026-11-18, N-073). Carte : N-072 (P1, S, decision). Outil de mesure : `cargo run --example hotkey_check` (Machine de référence MSI MS-7A71, Windows 10 Home).

## Question de la carte

Trouver une combinaison **une main**, facile, qui ne casse pas les apps web (Google Docs, Chrome/Edge devtools, etc.).

## Décision retenue (merge PR #59, squash `e079d91`)

**Défaut lire = `Win+Alt+S`** (1 seule main gauche, « S » pour Speak, libre sur la machine de référence mesurée avec Navi fermée). Implémenté dans `src-tauri/src/hotkey/mod.rs` (`register_defaults`) depuis le commit `4e054c3`.

> Historique : la première recommandation de cette étude était `Win+Alt+Espace` ; elle a été remplacée par `Win+Alt+S` avant merge. Ce document est aligné sur la décision réelle (carte N-073).

## Observations sur la machine de référence

Mesure faite avec `hotkey_check` (chaque combinaison enregistrée, process court, puis libérée à la sortie).

⚠️ Deux biais connus de la mesure, constatés le 2026-11-18 :
- quand Navi tourne, elle détient déjà `Win+Alt+S`, `Ctrl+Maj+P`, `Ctrl+Maj+S` → « ECHEC … HotKey already registered » est un artefact, pas un conflit OS ;
- la liste de candidats contient des doublons (`Ctrl+Maj+Espace` ×2, `Ctrl+Alt+S` ×2) → le second de chaque paire échoue intra-processus.

Mesure propre (Navi fermée) : **13 enregistrés, 3 échecs** — `Win+Ctrl+Espace` (réellement réservée) et les deux doublons intra-processus.

| Combinaison | Résultat machine | Commentaire |
|---|---|---|
| Ctrl+Maj+Espace (défaut initial) | ECHEC | déjà réservée par l'OS/une app |
| Ctrl+Alt+Espace | ECHEC | déjà réservée |
| Win+Ctrl+Espace | ECHEC | déjà réservée |
| Win+Alt+Espace | OK | disponible ; 1 main ; recommandation initiale, **non retenue** |
| Ctrl+Alt+E | OK | 1 combinaison simple |
| Ctrl+Alt+A | OK | |
| Ctrl+Alt+K | OK | |
| Ctrl+Alt+M | OK | |
| Ctrl+Alt+L | OK | |
| Ctrl+Alt+S | OK | disponible (hors doublon intra-processus) |
| Ctrl+Alt+P | OK | |
| Alt+Shift+Espace | OK | mais **Alt+Shift** change la langue du clavier Windows |
| **Win+Alt+S (lire)** | **OK** | **retenu** — 1 main, libre (mesurée Navi fermée) |

Résultat machines tests seulement ; les conflits peuvent varier sur d'autres machines — la détection se fait à l'enregistrement (TW-018) et l'humain doit signaler la sienne.

## Pourquoi les conflits web actuels posent problème

| App | Combinaison en conflit | Comportement |
|---|---|---|
| Chrome / Edge | Ctrl+Maj+Espace | ouvrir la console / devtools en mode raccourci |
| Google Docs | Ctrl+Maj+Espace | boîte de dialogue « nombre de mots » / actions clavier UI |
| Word / Outlook | — | pas d'interférence notable |
| Windows 10 | Ctrl+Maj+Espace | souvent réservée à l'OS ou un outil sur ce poste |

## Recommandation

**`Win+Alt+S`** : combinaison une-main-gauche sur une touche lettre naturelle, très peu utilisée par les apps (aucune UI commune n'exploite Win+Alt), libre sur la machine de référence, pas de conflit Windows connu (Win+Alt n'est pas un préfixe d'accessibilité OS standard).

Le processus retentera l'enregistrement sur la machine de l'humain et affichera `hotkey_error` si pris.

**Alternatifs acceptables si Win+Alt+S est pris chez l'humain** :
1. `Win+Alt+Espace` → OK ici.
2. `Ctrl+Alt+K` → lisible, libre ici.
3. `Ctrl+Alt+A` → très court, proche `Ctrl+A`.

## Préconisation

**Garder par défaut `Win+Alt+S`** ; l'humain peut surcharger via settings (TW-052, raccourcis configurables).

## Vérification terrain (2026-11-18, après décision)

- Bouton « Lire la sélection » : OK.
- Raccourci `Win+Alt+S` : échec initial (la cible recevait « Win+Alt+Ctrl+C », modificateurs encore enfoncés) — corrigé par `d7d07b9` (cible = dernière fenêtre non-Navi, activation forcée + attente réelle) puis `116cb09` (attente du relâchement des modificateurs via `GetAsyncKeyState` avant injection Ctrl+C). Vérifié OK par l'humain (bouton et raccourci).

## Traçabilité

- Méthode : `cargo run --example hotkey_check` (Navi fermée pour une mesure propre).
- Décision associée : TW-018 (Ctrl+Maj+… échec), TW-045 (câblage effectif).
- PR : #59 (fusionnée, squash `e079d91`) — notamment cette décision ; #58 (TW-041) pour le contexte audio.
- Alignement documentaire : carte N-073 (PR à la suite).
