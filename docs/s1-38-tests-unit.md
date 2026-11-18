# S1-038 — Tests ciblés nettoyage / segmentation / détection de langue (carte N-038)

Date : 2026-11-18. Carte : N-038 (P2, M). Modules : `src-tauri/tests/text_pipeline.rs` (nouveau fichier d'intégration crate-level).

## Contenu

**15 tests d'intégration** sur l'API publique de `navi_lib` (aucun périphérique, aucune inférence) :

| Groupe | Tests |
|---|---|
| Corpus Word/Chromium | fragments tableau, bold/italic, commentaires internes Chromium |
| Corpus pipeline FR/EN | paragraphes complets : clean → detect → segment (all segments ≤ 480 tokens, `Docteur Martin` présent) ; texte mixte FR/EN → `Mixed` |
| CRLF Windows | `\r\n` normalisés, plus de `\r` |
| RTF | `\b \b0 \i` gras/italique, mots de contrôle retirés |
| URLs en prose | supprimées, structure intacte |
| Propriétés | 100 phrases déterministes — segments bornés ; aucun mot perdu |
| Limites comportementales | HTML mal formée (`>` dans attribut) → pas de panic, texte présent ; `<script>` jamais fermé → ignoré sans panic ; mot de 5000 chars → scindé, tous segments ≤ 481 tokens ; `<img data:base64>` seule → erreur `NoText` claire |

**Correctif trouvé par les tests** : l'abréviation FR `Dr` était développée en `Docteur` sans le point, produisant `Docteur. Martin` ; corrigé en `Dr.` → `Docteur` (e.g. `"local. Dr. Martin"` → `"local. Docteur Martin"`).

## Vérification

```
cargo test → 66 passed (unit) + 15 passed (intégration text_pipeline) — 81 au total, 0 échec
```

## Limites documentées

1. Le corpus est en grande partie synthétique-mais-réaliste (fragments Word/Chromium de l'analyse S0-12). Le corpus Word véritable complet appartient à N-016 (rapport de compatibilité).
2. `proptest` n'a pas été ajouté (dépendance) ; les propriétés sont vérifiées sur 100 cas déterministes.
3. Aucune exécution sur Word/Chromium réel dans cet environnement — la recette humaine (protocole) reste l'autorité.

## Traçabilité

- Carte : N-038. Sources : `src-tauri/tests/text_pipeline.rs`, `cargo test`, `docs/s1-36-detection-langue.md`, `docs/s1-35-nettoyage-texte.md`.
- Critère de la carte : « tests unitaires ciblés (branches, parsing) » → couvert par 15 tests d'intégration + 16 existants sur les modules.
