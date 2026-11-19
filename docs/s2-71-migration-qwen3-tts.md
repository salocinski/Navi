# TW-071 — Migration Kokoro → Qwen3-TTS-0.6B-CustomVoice (Ryan voix officielle)

Date : 2026-11-19. Carte : N-071 (P1, L, tache). Dépend de : décision Ryan (N-070, `docs/decisions.md`), PR #59 (base code + capture/hotkey).

## Ce qui est livré (branche `feat/TW-071-qwen3-tts`)

**Architecture** — le moteur Qwen3-TTS (PyTorch/Transformers, pas ONNX) est intégré via un **sidecar Python persistant** plutôt que embarqué dans le process Rust :

```
Navi (Rust) ──stdin JSONL──▶ qwen_sidecar.py (venv : torch + qwen_tts)
             ◀─stdout JSONL── échantillons f32 mono 24 kHz base64
```

- `src-tauri/resources/qwen_sidecar.py` — charge le modèle **une fois** (device auto : CUDA bf16 sinon CPU float32 ; `sdpa` par défaut, repli `eager`), puis sert `{"id","text","language","speaker"}` → `{"id","ok","sample_rate","pcm_f32_base64"}`. La bannière des libs (flash-attn…) est redirigée vers stderr ; stdout ne porte que le protocole.
- `src-tauri/src/tts/qwen.rs` — `QwenTtsEngine` implémentant le contrat `TtsEngine` (name `qwen3-tts-0.6b-customvoice`, FR/EN, voix **Ryan** par défaut, `Drop` tue le sidecar). Chargement borné (`NAVI_QWEN_STARTUP_TIMEOUT_MS`, défaut 10 min — premier run = téléchargement ~2,3 Go depuis le cache Hugging Face).
- `src-tauri/src/tts/fallback.rs` — `ResilientEngine` : **Qwen → Kokoro, repli automatique**. Échec au chargement (venv absent, modèle introuvable, CUDA absent→CPU) **ou** à la première panne de synthèse → bascule Kokoro avec message clair (`eprintln!`). `NAVI_TTS_ENGINE` = `auto` (défaut : Qwen puis Kokoro), `qwen` (obligatoire), `kokoro` (Kokoro direct). Kokoro reste chargé paresseusement (à la première chute).
- La sélection de segments, la machine d'états et la file audio sont **inchangées** : la sortie Qwen est 24 kHz mono f32, comme Kokoro.
- `src-tauri/examples/qwen_check.rs` — vérification + mesure RTF (`cargo run --example qwen_check`).

## Utilisation en développement

```powershell
$env:NAVI_QWEN_PYTHON = "D:\Git\Projets\Navi\scratch\qwen_test312_cuda\Scripts\python.exe"
cargo run --example qwen_check --manifest-path src-tauri\Cargo.toml
```

Sortie constatée sur la machine de référence (MSI MS-7A71, **RTX 3060 Ti**, bf16, sdpa) :

| Texte | Audio | Synthèse | RTF |
|---|---|---|---|
| FR (texte Domio, 13,4 s) | 13,12 s | ~40 s | **3,05–3,84** |
| EN (même texte) | 10,64 s | ~27–38 s | **2,97–3,53** |
| Chargement modèle (cache HF) | — | ~14–17 s | — |

`eager` → `sdpa` : gain d'environ 20 % (3,84 → ~3,1 ; 3,74 → ~3,0).

## ⚠️ Point bloquant : RTF ≈ 3

Sur RTX 3060 Ti, la synthèse est ~3× plus lente que la lecture en temps réel : la lecture progressive (S0-03) avec Qwen serait largement en retrait. Kokoro (CPU) reste aujourd'hui le seul moteur capable de tenir la lecture progressive pour l'utilisateur final. La migration Qwen est donc **fonctionnelle mais non viable en UX** sur cette machine à ce stade.

Pistes avant de faire de Qwen le défaut produit :
1. `flash-attn` (attention mémoire/vitesse ; pas de wheel Windows stable pour torch 2.11 aujourd'hui, compilation nécessaire — pari risqué) ;
2. GPU plus rapide / VRAM plus grande (RTF scale avec le GPU) ;
3. génération plus rapide (top_k/température, `max_new_tokens`, distillation — hors périmètre) ;
4. accepter un délai de démarrage de segmentation (pré-générer les premiers segments avant la lecture) et tolérer la latence par segment.

## Recette humaine restante (avant clôture N-071)

1. Écoute de la voix Ryan sur texte réel (FR + EN) — les WAV de `qwen_check` sont dans le CWD de l'exécution.
2. Décision produit : `NAVI_TTS_ENGINE` par défaut = `auto` (Qwen d'abord) ou `kokoro` tant que RTF ≳ 1 ?
3. Vérifier le repli : `NAVI_TTS_ENGINE=kokoro` puis lecture bouton/raccourci (doit être identique à l'existant) ; puis forcer l'échec Qwen (NAVI_QWEN_PYTHON=chemin bidon) et vérifier que Kokoro prend le relais avec un message clair.
4. Latence perçue en lecture sur document réel.

## Traçabilité

- Décision produit Ryan : `docs/decisions.md` (N-070) ; ordre Kokoro « en prod jusqu'à validation » inchangé.
- Note : la décision N-070 donnait RTF comme indice à produire ici — c'est le cas (tableau ci-dessus). La validation reste ouverte.
