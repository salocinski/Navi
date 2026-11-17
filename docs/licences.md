# Audit de licences (S0-08a) — première passe

Statut : **non concluant**. Réalisé par un sous-agent via des résumés de pages (pas le texte brut des LICENSE, sauf mention). À revérifier sur les fichiers LICENSE avant toute distribution. Date : 2026-11-16.

| Artefact | Licence | Commercial / redistribution | Obligations / remarques | Source |
|---|---|---|---|---|
| Kokoro-82M (code + poids) | Apache-2.0 | Oui | Copie de la licence, NOTICE, indiquer les modifications | huggingface.co/hexgrad/Kokoro-82M |
| Voix FR `ff_siwis` | Poids Apache-2.0, **voix issue de SIWIS (CC BY 4.0)** | Pas d'interdiction relevée ; texte de licence SIWIS **non lu** | Attribution probable (citation Edinburgh DataShare) | VOICES.md ; datashare.ed.ac.uk/handle/10283/2353 |
| Voix EN (`af_heart`, `bf_*`) | Aucune licence propre listée (Apache-2.0 du modèle) | Pas de restriction relevée | Provenance des données d'entraînement non documentée | VOICES.md |
| Export ONNX (onnx-community) | Apache-2.0 | Oui | Licence amont StyleTTS2-LJSpeech non vérifiée | huggingface.co/onnx-community/Kokoro-82M-ONNX |
| kokoro-onnx (code) | MIT | Oui | Conserver copyright + texte MIT | github.com/thewh1teagle/kokoro-onnx |
| **espeak-ng** | **GPL-3.0-or-later** | Oui, sous GPL | **Copyleft fort** : lier ou embarquer libespeak-ng impose la GPL au binaire distribué | github.com/espeak-ng/espeak-ng |
| misaki (G2P) | Apache-2.0 | Oui | **Français non documenté** (EN, JA, KO, ZH, VI) | github.com/hexgrad/misaki |
| ONNX Runtime | MIT | Oui | ThirdPartyNotices non vérifié | github.com/microsoft/onnxruntime |
| crate `ort` | Apache-2.0 ou MIT | Oui | Conditions des binaires téléchargés non confirmées (page en 403) | github.com/pykeio/ort |
| Tauri 2 | Apache-2.0 ou MIT | Oui | Logo Tauri en CC-BY-NC-ND : ne pas l'utiliser | github.com/tauri-apps/tauri |
| rodio / cpal | Apache-2.0 ou MIT / Apache-2.0 | Oui | Notices | github.com/RustAudio |

## Risques
1. **espeak-ng (GPL-3+)** : bloque la redistribution fermée si la bibliothèque est liée ou embarquée. À trancher : appli sous GPL, appel d'un binaire séparé (analyse juridique requise), ou phonémiseur permissif.
2. **Français** : le chemin phonémisation FR n'est pas confirmé sans espeak-ng. Alternative permissive non trouvée (à chercher, lié à S0-02b).
3. **Voix `ff_siwis`** : attribution CC BY à prévoir ; lire le texte de licence SIWIS.
4. **Voix EN** : provenance non documentée.
5. Contradiction dans le README Kokoro (« non-copyrighted audio » vs données CC BY listées).

## À faire pour clore S0-08a
- [ ] Lire les fichiers LICENSE bruts de chaque artefact
- [ ] Lire la licence SIWIS
- [ ] Trouver un phonémiseur FR/EN permissif, ou décider de la GPL
- [ ] Relire `cargo tree` / `cargo about` quand les dépendances seront choisies
