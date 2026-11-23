# N-087 — Spike : Whisper Large v3 Turbo en local

Date : 2026-11-23 · Carte : N-087 · Epic Dictée vocale

## Protocole

- Modèle : `ggml-large-v3-turbo.bin` (non quantifié), 1 624 555 275 octets (1,51 Gio), SHA-256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`. Source : Hugging Face `ggerganov/whisper.cpp`.
- Moteur : binaires précompilés whisper.cpp b5907 (`whisper-cli`), variante CPU (`whisper-bin-x64`) et CUDA 12.4 (`whisper-bin-win-cuda-12.4.0-x64`). Réglages par défaut (beam 5), langue `auto`.
- Machine : i7-7700K (4 c / 8 t), 32 Go RAM, RTX 3060 Ti 8 Go.
- Audio : voix du propriétaire (français), 16 kHz mono, extraits de 5, 15, 24 et 60 s (le 60 s boucle l'enregistrement de 24 s).
- Mesures : durée totale du processus (chargement inclus), RAM de travail maximale, VRAM maximale (`nvidia-smi`, base 1 883 MiB déjà utilisée par le système).

## Résultats

| Extrait | GPU total | GPU hors chargement | CPU total | RAM max (GPU / CPU) | VRAM ajoutée (GPU) |
|---|---|---|---|---|---|
| 5 s | 3,8 s | 0,6 s | 79,7 s | 461 Mo / 1,8 Go | ~1,9 Gio |
| 15 s | 3,8 s | 1,1 s | 96 s | 485 Mo / 1,8 Go | ~1,9 Gio |
| 24 s | 4,5 s | 1,5 s | 101 s | 491 Mo / 1,8 Go | ~1,9 Gio |
| 60 s | 6,3 s | 3,6 s | 176 s | 557 Mo / 1,9 Go | ~2,0 Gio |

- Chargement du modèle : ~2,7 à 3,3 s (GPU comme CPU) — d'où le chargement à la demande (N-090) à garder chaud pendant une session de dictée.
- Détection de langue : `fr` (p = 0,999). Ponctuation et casse correctes, aucun repli.
- Qualité : transcription fidèle sur les 24 s ; seule réserve, le 60 s boucle le même texte, donc non représentatif de la variété du discours.

## Conclusions

1. **GPU : viable.** Transcription après relâchement en 0,6 s (5 s de parole) à 3,6 s (60 s), RAM ~0,5 Go, VRAM ~2 Gio. Un appui maintenu de 60 s coûte moins de 4 s d'attente après relâchement.
2. **CPU : inutilisable pour ce modèle non quantifié.** L'encodeur coûte ~38 s par fenêtre de 30 s quelle que soit la durée de la parole : 80 s pour 5 s dictées. Whisper traite l'audio par fenêtres de 30 s, donc le coût CPU croît par paliers.
3. **Durée maximale d'appui maintenu** : pas de limite imposée par la latence GPU ; proposition de plafond à 60 s (< 4 s de traitement), ajustable dans les réglages (N-093).
4. **Repli sans GPU compatible** : à décider. Un modèle plus petit (`small`/`base`) ou une quantification n'ont pas été mesurés dans ce spike. À mesurer avant de promettre la dictée sur machine sans GPU NVIDIA.
5. **Taille** : le modèle (1,51 Gio) tient sous la limite de 2 Go par fichier des releases GitHub : téléchargement au premier usage (N-094). Le runtime CUDA seul pèse ~700 Mo (cuBLAS 450 Mo + ggml-cuda 650 Mo décompressés) : à prendre en compte pour l'installeur et N-090.

## Limites du spike

- Anglais non mesuré (seul un enregistrement français était disponible) ; la détection auto FR/EN n'est pas validée sur de l'anglais.
- Un seul locuteur, un seul micro, pas de bruit ambiant.
- Backends CPU/CUDA testés via binaires officiels ; la compilation `whisper-rs` (CMake + MSVC) reste à valider par N-090.
- GPU non-NVIDIA (Vulkan, etc.) non testé.
