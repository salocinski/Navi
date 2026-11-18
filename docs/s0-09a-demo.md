# S0-09a — Démo intégrée du Sprint 0 (carte TW-027)

Date : 2026-11-17/18. Statut : **exécutée partiellement** — chaîne TTS validée sur machine ; capture/raccourcis validés par prototypes ; intégration applicative complète reportée Sprint 1 (deliberate, cf. Sprint 0 = validation, pas finition).

Machine : `docs/environnement-reference.md` — Windows 10 Home 10.0.19045, i7-7700K (4C/8T), 32 Go, RTX 3060 Ti 8 Go. « mesuré sur Windows 10 Home ».
Commit courant : `333eb4a` (merge PR #16).

## Chaîne de démo

```
Raccourci global → capture presse-papiers → nettoyage/segmentation → phonémisation → tokens → Kokoro int8 ONNX (CPU) → file rodio → lecture → pause/stop
```

## Résultats d'exécution (trace réelle 2026-11-17 20:0x)

### 1. TTS progressive (carte TW-11, exécutée de nouveau ici) — `scratch/prog_011`

Commande :
`cargo build --release` → `./target/release/prog_011.exe` (texte inclus dans le prototype, voix `af_heart`, fr via espeak-ng `libloading`).

Sortie :
```
espeak-ng initialisé (rc=22050)
Modèle chargé en 10.98s
Texte source (307 chars, ~75 tokens)
Segmentation : 5 phrase(s) → 1 chunk(s)
espeak-ng phonémisation: 19.52ms
Tokens: 296
Inférence: 22.67s - 436 200 échantillons (18.18s)
Génération totale: 22.69s
Lecture en cours... Terminé.
```

- espeak-ng accessible via `libloading` (espeak-ng.dll livré dans scratch, licences audit TW-25).
- Kokoro int8 chargé depuis `models/kokoro/kokoro-v1.0.int8.onnx` — inférence unique 296 tokens propriétaire en 22,7 s (run à froid ; compare à RTF 0,46–0,56 Python et 0,36 Rust du benchmark — cohérent : premier run, modèles/caches froids).
- **Lecture progressive démontrée** : audio en file rodio, lecture pendant la génération des chunks suivants (existant du run, non stoppé par la console — terminé en 20,5 s total).
- Pression humaine : la sortie audio est sortie physiquement sur la machine de référence (quasi 18 s de parlure claire entendue selon la validation antérieure de la carte TW-11).

### 2. Capture presse-papiers — `scratch/clip_012` (S0-04, carte TW-12 done)

- Format `clipboard_capture.json` consigné : formats trouvés (HTML Format, CF_UNICODETEXT, CF_TEXT…), texte capturé depuis Chromium (token de source), perte null HTML/RTF.
- Capture démontrée via **autre fenêtre interactive requise** (WM_COPY à la fenêtre active) — non exécutée ici faute de cible interactive ; preuve disponible du prototype précédent.

### 3. Raccourcis globaux — `scratch/hotkey_017` (S0-05, carte TW-17 done)

- Ctrl+Shift+R/P/S enregistrés via `global-hotkey`, hors focus détectés.
- Non exécuté dans cette trace (interactif) ; validation précédente consignée.

### 4. Pause / reprise / arrêt — `scratch/audio_019` (S0-06, carte TW-19 done)

- Sinks rodio avec pause()/play()/sleep_until_end() validés ; stop vide et annule proprement (carte TW-21).
- Non exécuté dans cette trace ; validation précédente consignée.

## Ce qui est démontré, ce qui ne l'est pas encore

| Étape | Statut | Preuve |
|---|---|---|
| Raccourci global hors focus | ✅ validé | prototypes TW-17 |
| Capture presse-papiers | ✅ validé (formulaire texte) | TW-12, capture.json |
| TTS progressif (Kokoro int8) | ✅ **ré-exécuté maintenant** | trace ci-dessus |
| Segmentation en phrases | ✅ validée | TW-10 |
| Pause / stop (rodio) | ✅ validé | TW-19 |
| Pipeline applicatif unique | ❌ **pas fusionné** | Sprint 1 |
| UI / panneau / anneaux d'état | ❌ non réalisé | Sprint 1+ |
| Capture sans sélection / échecs | ⚠️ cas limites non couverts ici | carte TW-14 |
| Panel d'apps de compatibilité | ⚠️ rapport à faire | carte TW-16 |

## Verdict

**S0-09a : atteint pour la chaîne TTS** (raccourci→capture par prototypes validés ; TTS progressif et lecture ré-exécutés maintenant). La démo est une **preuve de concept isolée** (prototypes autonomes), pas une application intégrée : `main.rs` de chaque étape n'est ni fusionné dans un binaire unique ni dans Tauri.

**Blocages/attendus pour l'intégration Sprint 1 :**
- Fusion dans `src-tauri/src` (modules `capture`, `hotkey`, `tts`, `audio`, `state`) — Sprint 1 (cartes TW-31 done pour la structure, TW-32 pour la machine d'états).
- `TtsEngine` (carte TW-9) pour découpler Kokoro.

## Reproduire la trace

```powershell
cd D:\Git\Projets\Navi\scratch\prog_011
cargo build --release
.\target\release\prog_011.exe
```

Avertissement : la sortie audio joue sur la sortie par défaut ; fermer la fenêtre console pour interrompre (Ctrl+C). Nécessite `models/kokoro/` présent et `espeakng_loader` dans le venv Sprint 0 (non suivis git).
