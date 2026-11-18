# S1-014 — Cas limites de capture (carte TW-014)

Date : 2026-11-18. Carte : N-014 (P1, M). Module : `src-tauri/src/capture/mod.rs`.

## Cas limites et erreurs renvoyées

| Cas | Mécanisme | Erreur (`CaptureError`) |
|---|---|---|
| Aucune fenêtre au premier plan | `GetForegroundWindow` NULL | `NoForegroundWindow` |
| Application gelée / copie bloquée (WM_COPY non traité dans 1 200 ms) | `SendMessageTimeoutW` + `SMTO_ABORTIFHUNG` (au lieu de `SendMessageW` bloquant) | `ApplicationHung` |
| Copie asynchrone lente | Attente jusqu'à 1 500 ms via `GetClipboardSequenceNumber` (sondage 25 ms), bornée | — (continue l'attente) |
| Aucune sélection / copie non effectuée (le presse-papiers ne change pas) | Délai de 1 500 ms dépassé, séquence inchangée | `NoSelection` |
| Presse-papiers verrouillé par une autre application | `OpenClipboard` avec 5 tentatives × 100 ms | `ClipboardBusy` |
| Aucun format texte exploitable / texte vide | Priorité Unicode > HTML > RTF > ANSI, trim vide | `NoTextContent` |
| Interruption (fenêtre détruite pendant la capture) | Échec de `SendMessageTimeoutW` | `ApplicationHung` |
| Format illisible (GlobalLock/GlobalSize NULL, UTF-16 invalide) | Erreur formatée par format, on tente le suivant | (dégradé, sinon `NoTextContent`/`ReadFailure`) |

Chaque variante produit un message français exploitable par l'UI (cf. test `messages_derreur_clairs_pour_chaque_cas`).

## Changements de comportement par rapport à la version précédente

1. `SendMessageW` (bloquant sans fin) → `SendMessageTimeoutW` avec `SMTO_ABORTIFHUNG` et 1 200 ms.
2. Délai fixe de 200 ms → attente dynamique sur le numéro de séquence du presse-papiers (jusqu'à 1 500 ms).
3. `OpenClipboard` unique → 5 tentatives.
4. `CaptureResult::error(String)` → `CaptureResult::failure(CaptureError)` avec messages normalisés ; `Display` implémenté pour chaque cas.
5. Signature publique `capture_selection() -> CaptureResult` inchangée.

## Vérification

```text
cargo test --manifest-path src-tauri/Cargo.toml
test result: ok. 16 passed; 0 failed (capture 4 : messages clairs, séquence détectée/inchangée, result)
```

## Limites / à acter

1. **La restauration du presse-papiers n'est pas couverte** ici (la capture écrase toujours le contenu utilisateur) : carte N-013 reportée → N-043. Risque UX/données connu et accepté pour l'instant.
2. La détection « aucune sélection » repose sur le numéro de séquence : une application qui copie **exactement le même contenu** modifie quand même le numéro de séquence (SetClipboardData), donc le faux positif est improbable mais possible si une app re-définit le presse-papiers hors sélection utilisateur (rare, documenté).
3. `SMTO_ABORTIFHUNG` ne couvre que les apps gelées (messages non traités) ; une app qui traite WM_COPY mais où `Ctrl+C` est ignoré échoue en `NoSelection` — c'est le comportement souhaité.
4. La détection d'interruption utilisateur en vol (Esc) n'existe pas côté Windows sans hook bas niveau ; les timeouts bornent tous les cas d'attente.
5. Les tests unitaires couvrent la logique pure (séquence, messages) ; la chaîne complète (presse-papiers réel, WM_COPY réel) nécessite un test manuel : sélection dans Chromium/Word → capture ; aucune sélection ; app bloquante. À consigner au rapport de compatibilité (N-016).

## Traçabilité

- Carte : N-014. Sources : `src-tauri/src/capture/mod.rs`, `cargo test` (ci-dessus).
- Critère de la carte : « Chaque cas renvoie une erreur claire » → couvert par `CaptureError::Display` + test.
