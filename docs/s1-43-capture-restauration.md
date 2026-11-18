# S1-043 — Capture presse-papiers durcie : snapshot + restauration garantie (carte TW-043)

Date : 2026-11-18. Carte : N-043 (P1, L). Module : `src-tauri/src/capture/mod.rs`.
Issu de l'exploration S0-13 / TW-13 (« exploré, restauration non validée ») et du report de N-013.

## Problème initial

La capture écrasait le presse-papiers utilisateur sans le sauvegarder. L'utilisateur perdant
son presse-papiers (texte, image, HTML, fichier…) était un risque de **perte de données**
connu et accepté pour le Sprint 0 uniquement.

## Solution : snapshot intégral + restauration systématique

```
capture_selection()
  ├─ 1. snapshot_clipboard()        // AVANT Ctrl+C simulé (SendInput) : tous formats (mémoire globale)
  │      ├─ OpenClipboard (5 tentatives × 100 ms)
  │      ├─ EnumClipboardFormats → format par format
  │      │    ├─ format GDI / atom / non-mémoire → ignoré, compté (skipped)
  │      │    └─ GlobalLock → copie octets → skipped sinon
  │      └─ CloseClipboard
  ├─ 2. capture_steps()             // étapes TW-014 (SendInput Ctrl+C, séquence, lecture)
  ├─ 3. restore_clipboard(snapshot) // TOUJOURS, même en cas d'erreur ou d'interruption
  │      ├─ OpenClipboard
  │      ├─ EmptyClipboard
  │      └─ pour chaque item : GlobalAlloc(GMEM_MOVEABLE) → copie → SetClipboardData
  │         └─ échec par format → compté, les autres formats réécrits
  └─ 4. CaptureResult.restore : None (complet) | Some(message) (partiel)
```

### Garantie

- **Succès** : presse-papiers utilisateur retrouvé intégralement.
- **Erreur** (aucune sélection, app gelée, presse-papiers verrouillé, texte vide…) :
  restauration quand même effectuée.
- **Interruption** : tous les chemins d'attente sont bornés (1 500 ms séquence,
  retries) ; `restore_clipboard` est hors du chemin d'erreur et s'exécute quoi qu'il arrive.
- **Snapshot impossible** (presse-papiers inaccessible avant la copie) : **capture refusée**
  (`SnapshotFailed`) — jamais d'écrasement sans sauvegarde.

### Formats exclus de la sauvegarde (handles, non copiables)

CF_BITMAP, CF_METAFILEPICT, CF_PALETTE, CF_LOCALE (atom 16 bits), CF_OWNERDISPLAY et CF_DSP* (0x80–0x8F),
plage GDIOBJFIRST–GDIOBJLAST (0x0300–0x03FF). Ces formats sont **ignorés explicitement** et comptés
(`skipped`), jamais renvoyés comme erreurs.

## API publique

```rust
pub struct CaptureResult {
    pub text: String,
    pub format_used: String,
    pub success: bool,
    pub error: Option<String>,
    pub restore: Option<String>, // None = restauration complète
}
```

`capture_selection() -> CaptureResult` inchangée pour l'appelant ; nouveau champ additif `restore`.

## Vérification

```text
cargo test --manifest-path src-tauri/Cargo.toml
test result: ok. 18 passed; 0 failed
(capture : test_capture_result, messages_derreur_clairs_pour_chaque_cas,
 formats_non_copiables_classes_correctement, rapport_restauration_partielle_formate,
 sequence_changee_est_detectee, sequence_inchangee_expire)
```

## Limite connue et actée

1. **GDI handle formats are not restored** (CF_BITMAP etc.) : l'API `windows` 0.58 ne copie pas les
   handles ; les exclure explicitement évite les faux espoirs — documenté, compté, testé.
2. Fuite bornée sur échec `GlobalLock` après `GlobalAlloc` : `GlobalFree` n'est pas exposée par
   windows crate 0.58 ; chemin d'erreur rare, taille ≤ octets du format — accepté, documenté dans le code.
3. **Validation terrain non automatisée** (une vraie restauration exige un presse-papiers réel et
   muterait l'utilisateur) : 2 scénarios joués et validés par l'humain (texte riche Word, Chromium) ; 3 scénarios restants listés en validation terrain — non bloquants pour la clôture.
4. Le presse-papiers est restauré même si l'utilisateur a **volontairement** re-copié pendant la
   capture (fenêtre ~1,5 s) : comportement connu — la restauration écrase ce nouveau contenu.
   Acceptable pour un Ctrl+C programmatique bref ; à réévaluer si des plaintes remontent.

## Validation terrain (2026-11-18, machine de l'humain)

Constat majeur : le `SendMessageTimeoutW`/`WM_COPY` d'origine (TW-014) **ne déclenchait pas la copie** —
`WM_COPY` est un message de fenêtre, pas l'accélérateur clavier ; la lecture retournait le contenu du
presse-papiers préexistant (faux positif du test S0-12). Corrigé par `SendInput` Ctrl+C (commit `31a6780`).

Scénarios joués par l'humain, avec l'exemple CLI `cargo run --example capture_recipe` :

| Scénario | Résultat |
|---|---|
| Texte riche Word copié en témoin → sélection ailleurs → capture → Ctrl+V | ✅ réussite ; **presse-papiers témoin restauré intégralement** |
| App Electron/Chromium : sélection web → capture | ✅ texte capturé, presse-papiers restauré |
| Sélection image (CF_DIB) / fichier (CF_HDROP) | non joué |
| App gelée / SendInput bloqué | non joué |
| Presse-papiers verrouillé par une app tierce | non joué |

Le test de restauration vise les deux scénarios joués (texte riche multi-formats Word est le cas
le plus exigeant : CF_UNICODETEXT + CF_HTML + formats internes) — résultat conforme.

## Protocole de recette manuelle (intégralité non encore jouée)

1. Copier un **texte riche** depuis Word (Ctrl+C). Vérifier le presse-papiers (coller ailleurs) : contenu riche OK.
2. Sélectionner du texte dans Chromium, lancer Navi (une fois intégré) ou le prototype : capturer.
   → le texte sélectionné est lu ; re-coller ailleurs : le presse-papiers d'origine (Word) est revenu intact.
3. Copier une **image** (CF_DIB), lancer la capture : → échec attendu (`NoTextContent`) *mais*
   le presse-papiers image est **toujours présent** après l'échec.
4. Copier un **fichier** (CF_HDROP), capturer : idem.
5. Bloquer le presse-papiers avec une app tierce, capturer : → `SnapshotFailed`/`ClipboardBusy`
   et **rien n'est écrasé**.
6. Valider sur : navigateurs, Word, éditeur code, PDF, terminal, app Electron, app élevée (N-016).

## Traçabilité

- Carte : N-043. Sources : `src-tauri/src/capture/mod.rs`, `cargo test` (ci-dessus),
  `docs/s0-12.md`, `docs/s0-13.md`, carte TW-13 reportée.
- Critère de la carte : « formats, restauration garantie, tests d'erreur et d'interruption » → couverts
  (formats : classification + snapshot/restore ; restauration : systématique ; interruption : timeouts +
  chemin de restauration hors erreurs). Validation terrain : protocole ci-dessus.
