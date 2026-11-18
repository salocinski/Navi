# S1-037 — Segmentation en production pour Kokoro (carte N-037)

Date : 2026-11-18. Carte : N-037 (P1, M). Module : `src-tauri/src/text/segmentation.rs`.
Héritée du prototype S0-10 (scratch/seg_010), durcie pour la production.

## Stratégie

```
clean_captured_text (N-035) → segment_text()
   1. split_sentences : ponctuation forte (. ! ?) suivie d'espace/fin, sauts de ligne = frontière
   2. is_abbreviation : listes FR/EN (M., Dr., Mme, etc., p.ex.) + initiale majuscule (J. R. R. Tolkien)
      (plus de règle « ≤ 4 lettres » — éliminait les fausses coupures « Un. »/« ici. »)
   3. chunk_by_token_limit : regroupement ≤ 480 tokens estimés (4 chars ≈ 1 token ; marge sous 510)
   4. segment isolé > limite → découpage aux frontières de mots ; un mot > limite → découpage en dur
```

## API

```rust
pub const MAX_TOKENS_PER_CHUNK: usize = 480;
pub struct Segment { pub index: usize, pub text: String, pub estimated_tokens: usize }
pub fn segment_text(text: &str) -> Vec<Segment>
pub fn estimate_tokens(text: &str) -> usize
pub fn split_sentences(text: &str) -> Vec<String>
pub fn chunk_by_token_limit(sentences: &[String], max_tokens: usize) -> Vec<String>
pub fn chunk_plain_text(text: &str, max_chars: usize) -> Vec<String>
```

La sortie (`Vec<Segment>`) est la **file de segments** : chaque `Segment` → une inférence Kokoro
(N-039) → un chunk audio (file ordonnée du lecteur). L'intégration pipeline arrive avec N-039/N-041.

## Limites documentées

1. Estimation 1 token ≈ 4 chars (FR/EN) — approximation conservatrice ; tokens réels comptés à la tokenisation Kokoro (N-039 validera sur corpus réel).
2. Pas de détection de langue par phrase — détection globale N-036 en amont, texte supposé homogène.
3. Pas de `unicode-segmentation` : ponctuation forte ASCII, compteurs en chars — suffisant pour FR/EN ;
   les graphemes exotiques ne coupent rien (ponctuation CJK = pas de frontière, comportement voulu).
4. Abréviations : listes FR/EN explicites + initiale ; les abréviations sans point final des deux côtés (ex. « dr ») ne sont pas reconnues — acceptable.
5. Un mot > 1920 chars (2500 chars contigus, ex. fichier collé) est scindé en dur — préférable à
   laisser dépasser la fenêtre Kokoro (le contrat « estimated_tokens ≤ 480 » prime sur l'intégrité du mot).

## Vérification

```
cargo test → 66 passed, 0 failed
(phrases_simples, abreviation_protegee [Dr./etc./M.], sauts_de_ligne, estimate_tokens,
 chunk_limit_respectee [2500 chars contigus → 2+ chunks ≤ 480], frontieres_de_mots,
 texte_long_en_deux_chunks [35 phrases → 2 chunks], ordre stable, texte vide, jointure préservée)
```

## Traçabilité

- Carte : N-037. Sources : `src-tauri/src/text/segmentation.rs`, `cargo test`, `docs/s0-10.md` (prototype).
- Critère de la carte : segmentation en production + préparation de la file ✓ (la file est `Vec<Segment>` prête à l'injection dans l'inférence).
