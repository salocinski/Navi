# N-094 — Modèle de dictée et DLL CUDA

## Modèle (téléchargé, non embarqué)

- `ggml-large-v3-turbo.bin`, 1 624 555 275 octets, SHA-256 `1fc70f77…e2bc69` (constantes dans `src-tauri/src/stt/model.rs`).
- Emplacement : `%APPDATA%\Navi\models\ggml-large-v3-turbo.bin`. `NAVI_STT_MODEL` le remplace (développement : ni vérifié ni téléchargé).
- Au premier usage de la dictée : modèle absent ou corrompu (taille puis SHA-256, ~4 s, une fois par lancement) → téléchargement en tâche de fond depuis Hugging Face (`.part` puis renommage), dictée refusée avec un message clair jusqu'à la fin.
- Événements `dictation-state` : `model-download {done,total}` (par pour cent), `model-ready`, `model-failed {message}`. Commandes : `dictation_model_status`, `download_dictation_model`. L'interface arrive avec N-093.
- Mesuré : téléchargement réel de 1,5 Go en ~15 min sur le poste de dev, empreinte conforme (test ignoré `telechargement_reel`).

## DLL CUDA (embarquées)

- L'exécutable importe `cublas64_13.dll` (qui tire `cublasLt64_13.dll`, ~530 Mo) ; `cudart` est lié en statique.
- `node tools/copy-cuda-dlls.mjs` les copie dans `src-tauri/cuda-redist/` (ignoré par git) ; `tauri.bundle.conf.json` les place à côté de l'exécutable.
- Build de livraison : ne pas fixer `CMAKE_CUDA_ARCHITECTURES=86` (réservé au développement) ; viser plusieurs architectures, sinon le GPU d'un autre utilisateur n'est pas supporté.

## Limites assumées

- Pas de reprise d'un téléchargement interrompu ni de délai sur le corps (`shortcut:` dans `model.rs`).
- Installateur avec DLL non construit ni testé ici ; absence de pilote NVIDIA ou de GPU : comportement à mesurer en N-096.
- Mentions légales (Whisper, whisper.cpp, whisper-rs, CUDA) ajoutées à `THIRD_PARTY_NOTICES.md` ; la conformité au contrat NVIDIA reste à valider par le propriétaire.
