# S1-036 — Détection automatique FR/EN (carte N-036)

Date : 2026-11-18. Carte : N-036 (P1, M). Module : `src-tauri/src/text/mod.rs` (`language`), `src-tauri/src/utils/mod.rs`.
Exigence de la carte : détection **indépendante du moteur TTS** ; comportement défini pour langue inconnue ou texte mixte.

## Contrat

```rust
pub enum LanguageDetection { French, English, Mixed, Unknown }
pub fn detect_language(text: &str) -> LanguageDetection
impl LanguageDetection {
    pub fn label(&self) -> &'static str        // "fr" | "en" | "mixed" | "unknown"
    pub fn as_tts_hint(&self) -> &'static str  // "fr" | "en" — voir politique ci-dessous
}
pub(crate) utils::detect_language : déléguée, compat héritée ("fr" par défaut).
```

## Algorithme (hors moteur, déterministe, testable)

1. **Accents FR** (é è ê ë à â ä ç î ï ô ö ù û ü ÿ œ æ) × 3.
2. **Stopwords** des deux langues (~60 chacun, occurrences × 2).
3. **Apostrophes** typographiques (l', d', n'…) × 1,5 (indice français indirect).
4. Gagnant = score le plus élevé ; **`Mixed`** si le perdant ≥ 35 % du gagnant (aucun gagnant clair) ; **`Unknown`** si < 3 tokens ou score 0.

## Comportement pour langue inconnue ou texte mixte (exigence)

**Décision produit** : `Mixed` ou `Unknown` → **hint TTS = « fr »** (cible produit).
Raison : Navi est livré pour une audience FR ; forcer `en` sur un texte ambigu risquerait une phonémisation exotique sur du français.
L'utilisateur pourra forcer la langue explicitement (N-053 « Choix voix/vitesse/volume » et N-052 raccourcis configurables) — ce hint n'est qu'un défaut.

## Limites documentées

1. Heuristique : ne remplace pas un détecteur statistique (langdetect/whatlang) — suffisant pour FR vs EN ; aucun 3e langue (espeak gère d'autres voix mais hors contrat produit).
2. Texte mixte à dominance anglaise technique (ex. code/commentaires) → `Mixed` souvent → hint `fr` — assumé, corrigible par N-053.
3. URLs/paths strippés en amont (N-035) — la détection bénéficie du même nettoyage.
4. `as_tts_hint` ne fait **pas** varier le vocabulaire Kokoro, uniquement la phonémisation espeak (via le paramètre `language` — câblage dans N-039/N-041).
5. Pas de détection incrémentale / streaming ; un appel par segment de lecture (N-041).

## Vérification

```
cargo test → 56 passed, 0 failed
- language_francais_pur / anglais_pur / francais_accents_fortes / texte_mixte / trop_court_ou_sans_indice / tts_hint
- test_detect_language (utils, compat héritée) : inchangé, vert
```

## Traçabilité

- Carte : N-036. Sources : `src-tauri/src/text/mod.rs`, `src-tauri/src/utils/mod.rs`, `cargo test`.
- Critère de la carte : détection FR/EN hors moteur ✓ ; comportement inconnu/mixte défini et documenté ✓.
