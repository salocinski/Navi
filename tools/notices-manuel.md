# Mentions légales des composants tiers — Navi

Navi est distribué sous licence **GPL-3.0-or-later** (texte complet : fichier `LICENSE`, installé avec l'application). Il embarque les composants tiers listés ci-dessous, qui restent soumis à leurs propres licences. Ce fichier est généré : voir `tools/gen-notices.mjs` (partie rédigée : `tools/notices-manuel.md`).

Navi fonctionne **100 % en local** : aucun texte ni aucun son n'est envoyé hors de l'ordinateur.

## Moteur de synthèse vocale et modèles

| Composant | Auteur / source | Licence |
|---|---|---|
| Kokoro-82M — poids `kokoro-v1.0.int8.onnx` et styles de voix `voices-v1.0.bin` | hexgrad — https://huggingface.co/hexgrad/Kokoro-82M ; export ONNX : kokoro-onnx de thewh1teagle — https://github.com/thewh1teagle/kokoro-onnx | Apache-2.0 (poids) ; MIT (kokoro-onnx) |
| `vocab.json` (vocabulaire de phonèmes) | Fourni avec les artefacts Kokoro ci-dessus | Apache-2.0 |
| ONNX Runtime (binaire précompilé lié via la crate `ort`) | Microsoft — https://github.com/microsoft/onnxruntime | MIT ; notices de ses propres composants : https://github.com/microsoft/onnxruntime/blob/main/ThirdPartyNotices.txt |
| espeak-ng (phonémisation FR/EN, `espeak-ng.dll` et `espeak-ng-data`) | Jonathan Duddington, Reece H. Dunn et les contributeurs d'espeak-ng — https://github.com/espeak-ng/espeak-ng | GPL-3.0-or-later |

Le texte intégral des licences Apache-2.0 et MIT figure plus bas, parmi les textes de licence des dépendances.

### espeak-ng — offre de code source (GPL-3.0)

Le binaire d'espeak-ng est fourni avec Navi sous GPL-3.0-or-later. Son code source correspondant est disponible à l'adresse https://github.com/espeak-ng/espeak-ng ; sur demande adressée au dépôt de Navi (https://github.com/salocinski/Navi), le code source de la version distribuée est fourni.

## Voix

Le fichier `voices-v1.0.bin` contient plusieurs voix. Aucune des voix ci-dessous n'est enregistrée par Navi.

- **`ff_siwis`** (voix française, par défaut pour le français) : fondée sur le corpus *SIWIS French Speech Synthesis Database* — Honnet, Lazaridis, Garner, Yamagishi (2017), Université d'Édimbourg, https://datashare.ed.ac.uk/handle/10283/2353 — licence **Creative Commons Attribution 4.0 International (CC BY 4.0)** : https://creativecommons.org/licenses/by/4.0/. Des modifications ont été apportées (entraînement du modèle de synthèse Kokoro à partir du corpus).
- **Voix anglaises** (`af_heart`, `af_bella`, `af_nicole`, `af_sarah`, `af_sky`, `am_adam`, `am_michael`, `bf_emma`, `bf_isabella`, `bm_george`, `bm_lewis`) : publiées avec Kokoro sous Apache-2.0 ; aucune attribution tierce n'est exigée par la publication amont (VOICES.md de Kokoro). La provenance des données d'entraînement de ces voix n'est pas documentée en amont.

## Éléments volontairement absents

Le logo Tauri (CC BY-NC-ND) n'est pas utilisé.
