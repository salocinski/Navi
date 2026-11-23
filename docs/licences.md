# Audit de licences (S0-08) — passe 2, verdict

Date : 2026-11-17. Méthode : **fichiers LICENSE bruts lus** aux URL citées (passe 1 du 2026-11-16 s'appuyait sur des résumés de pages). Licence du projet tranchée : **GPL-3.0-or-later** (dépôt `salocinski/Navi`, fusion de la PR #1).

## Cadre retenu

Navi est distribué sous **GPL-3.0-or-later** via des releases GitHub. Conséquences directes :

- **espeak-ng (GPL-3.0+) est embarquable et liable sans contournement** — la voie « binaire séparé / sidecar » n'est plus nécessaire ;
- les dépendances Apache-2.0 / MIT (Kokoro, ONNX Runtime, Tauri…) sont compatibles avec une œuvre GPL dans ce sens ;
- l'œuvre distribuée (installeur) portant la GPL, les mentions tierces restent à recopier (Apache NOTICE de Kokoro, notices MIT) — voir carte TW-61.

## Tableau par artefact

| Artefact | Licence (fichier brut lu) | Compat. GPL-3.0 | Obligations / remarques |
|---|---|---|---|
| Code Navi | **GPL-3.0-or-later** (choix projet, PR #1) | — | Source publié, mentions copyright |
| Kokoro-82M (code) | Apache-2.0 — `hexgrad/kokoro` LICENSE | ✅ | Copie licence + NOTICE si présent, indiquer les modifications |
| Kokoro-82M (poids `kokoro-v1.0.int8.onnx`) | Apache-2.0 (publication HF `hexgrad/Kokoro-82M`) | ✅ | Idem |
| `voices-v1.0.bin` (conteneur) | Apache-2.0 (artefact de la même publication) | ✅ | Ne pas embarquer `ff_siwis` (voir ligne suivante) |
| Voix `af_heart` (retenue, féminine) | Sous Apache-2.0, **aucune attribution CC BY listée** (VOICES.md) | ✅ | Provenance des données d'entraînement non documentée — à mentionner honnêtement |
| Voix `am_adam` (retenue, masculine) | Sous Apache-2.0, aucune attribution CC BY listée | ✅ | Notée **F+ / qualité D en amont** (VOICES.md). Choix humain validé à l'écoute (carte TW-29) — à reconsidérer si l'oreille change : am_michael, am_fenrir, am_puck notés C+ |
| Voix `ff_siwis` (**embarquée**, voix française par défaut depuis N-053) | Poids Apache-2.0, voix **CC BY 4.0 (SIWIS, Edinburgh DataShare)** | ✅ | **Attribution CC BY 4.0 requise** : ajoutée à `THIRD_PARTY_NOTICES.md` (N-061). Supersede la décision « hors distribution » de la passe 2 |
| kokoro-onnx (prototype Python) | MIT — `thewh1teagle/kokoro-onnx` LICENSE | ✅ | Conserver copyright + texte MIT |
| espeak-ng (phonémisation FR/EN) | **GPL-3.0-or-later** — `espeak-ng/espeak-ng` COPYING | ✅ (homogène) | Lié directement ; crédits dans les mentions tierces |
| misaki (G2P EN/JA/KO/ZH/VI) | Apache-2.0 | ✅ | **Pas de français** (README vérifié le 2026-11-17) — pas dans la chaîne FR |
| ONNX Runtime | MIT — `microsoft/onnxruntime` LICENSE | ✅ | ThirdPartyNotices à recopier |
| crate `ort` (Rust) | Apache-2.0 **ou** MIT (LICENSE-MIT lu) | ✅ | À reconfirmer via `cargo about` quand les crates seront figées |
| Tauri 2 | Apache-2.0 / MIT (LICENSE-APACHE-2.0 lu) | ✅ | **Logo Tauri en CC-BY-NC-ND : ne pas l'utiliser** dans la distribution |
| rodio | MIT (LICENSE-MIT lu) | ✅ | Notices |
| cpal | Apache-2.0 (LICENSE lu, dual avec MIT) | ✅ | Notices |

## Risques restants (honnêteté de l'audit)

1. **Provenance des voix EN** : les données d'entraînement de `af_heart` et `am_adam` ne sont pas documentées amont (« non-copyrighted audio » du README Kokoro). Aucune attribution exigée, mais la traçabilité est faible — noté, non bloquant.
2. **`am_adam` est la voix masculine EN la moins bien notée amont (F+)**. Le choix est validé à l'oreille humaine ; la carte TW-29 conserve les alternatives générées en secours.
3. Contradiction du README Kokoro (« non-copyrighted audio » vs voix CC BY listées) : documentée, sans impact pour notre sélection.
4. Les binaires téléchargés par `ort` (ONNX Runtime précompilé) devront être re-vérifiés via `cargo about` / `cargo tree` quand les dépendances Rust seront figées.

## Clôture et renvois

- [x] Fichiers LICENSE bruts de chaque artefact lus (URL dans le tableau)
- [x] Licence SIWIS : `ff_siwis` désormais embarquée → attribution CC BY 4.0 dans `THIRD_PARTY_NOTICES.md` (N-061)
- [x] Phonémiseur : espeak-ng conservé, légitimé par le choix GPL-3.0
- [x] Inventaire des dépendances → `THIRD_PARTY_NOTICES.md`, généré par `tools/gen-notices.mjs` (N-061) ; installeur : N-060
- [x] Décision de redistribution → carte TW-26 (clôturée le 2026-11-17)
