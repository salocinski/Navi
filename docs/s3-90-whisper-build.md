# N-090 — Whisper derrière `SttEngine` : build et exécution

## Code
- `src-tauri/src/stt/mod.rs` : trait `SttEngine` (`load` / `unload` / `transcribe`), `SttError`, `Transcription`, `pick_language` (FR/EN seulement), `join_segments`.
- `src-tauri/src/stt/whisper.rs` : `WhisperEngine` (whisper.cpp via `whisper-rs` 0.16, modèle `ggml-large-v3-turbo.bin`, beam 5).
- Feature Cargo `cuda` (défaut). La CI compile en `--no-default-features` (whisper.cpp CPU, sans CUDA Toolkit).

## Résultat mesuré (machine de dev, RTX 3060 Ti, voix du propriétaire, 24 s de français)
Transcription 1,48 s après chargement, langue détectée `fr`, texte complet avec ponctuation. Identique au spike N-087.

## Prérequis de build avec CUDA (Windows)
- CMake (≥ 4) dans le PATH.
- CUDA Toolkit 13.5 (VS 2026 / MSVC 14.51 exige CUDA 13.x). Composants minimaux : `nvcc`, `crt`, `nvvm`, `nvptxcompiler`, `cudart`, `cublas`, `cublas_dev`, `nvrtc`, `nvrtc_dev`, `cccl`, `visual_studio_integration`.
  Installation silencieuse : `setup.exe -s -n nvcc_13.5 crt_13.5 …` (extraire l'installeur sur un autre disque si `C:` est juste).
- Variables : `CUDA_PATH` et `CUDA_PATH_V13_5` = `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.5`.
- `BINDGEN_EXTRA_CLANG_ARGS` = `-I"<MSVC>/include" -I"<Windows Kits>/10/Include/<ver>/ucrt"` : sans cela `bindgen` ne trouve pas `stdbool.h` et retombe sur des liaisons périmées (erreurs de taille de structure).
- Dev : `CMAKE_CUDA_ARCHITECTURES=86` limite la compilation à l'architecture locale (première compilation ~10 min). Les builds de livraison doivent viser plusieurs architectures.

## Exécution
Les DLL CUDA (`cudart64_13`, `cublas64_13`, `cublasLt64_13`, ~530 Mo) sont dans `bin\x64` du toolkit et doivent être trouvables au lancement (PATH en dev). Leur livraison à l'utilisateur est traitée par N-094 / l'installeur.

## Limites assumées
- Pas de vérification SHA-256 du modèle au chargement (N-094) ; CUDA actif non contrôlé (repli CPU silencieux de whisper.cpp, ~80 s par phrase ; N-096).
- Voie CUDA non vérifiée en CI.
- Mentions légales à compléter (N-061) : whisper-rs (Unlicense), whisper.cpp et poids (MIT), runtime CUDA (EULA NVIDIA, redistribuable).
