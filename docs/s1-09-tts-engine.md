# S1-009 — Esquisse de l'interface `TtsEngine` (carte N-009)

Date : 2026-11-18. Carte : N-009 (P2, S). Modules : `src-tauri/src/tts/mod.rs`, `src-tauri/src/tts/engine.rs`.
Objectif de la carte : **isoler le moteur pour pouvoir le remplacer** (Kokoro actuel ; Piper et MeloTTS en réserve).

## Ce qui est posé

### Contrat (`src-tauri/src/tts/engine.rs`)

```rust
pub trait TtsEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn supported_languages(&self) -> &'static [TtsLanguage];
    fn voices(&self) -> Vec<String>;
    fn load(&mut self) -> Result<(), TtsError>;   // idempotent, chargement paresseux
    fn is_loaded(&self) -> bool;
    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisOutput, TtsError>;
}

pub struct SynthesisRequest { pub text: String, pub voice: Option<String>,
                              pub language: Option<TtsLanguage>, pub speed: f32 }
pub struct SynthesisOutput { pub samples: Vec<f32>, pub sample_rate: u32 }
pub enum  TtsLanguage { French, English }
pub enum  TtsError { NotLoaded, UnsupportedLanguage, UnknownVoice, ModelMissing,
                     Phonemization, Inference, Io } // Display + Error, messages FR clairs
pub const TTS_SAMPLE_RATE: u32 = 24_000;
```

### Implémentation Kokoro (fait, minimal)

- `tts::KokoroEngine` (ex-`TtsEngine`, renommée) implémente le contrat : `name = "kokoro-82m"`,
  langues FR/EN, `load()` = `ensure_session` (chargement paresseux inchangé), `is_loaded()` = session présente.
- `synthesize` délègue à l'inférence existante (phonémisation espeak → tokens → style → ONNX → `Vec<f32>`).
- Réexport `tts::{SynthesisRequest, SynthesisOutput, TtsError, TtsLanguage, TTS_SAMPLE_RATE, TtsEngine}`.
- Le trait est **utilisé par le prototype** (tests `tts::engine::tests::*` : stub engine charge → synthétise ; langue inconnue refusée).
- Le flux de production (file audio, N-041) n'est pas encore branché — c'est N-039/N-041.

## Est-ce que ça tient pour Piper / MeloTTS ?

| Critère | Piper (réserve) | MeloTTS (réserve) |
|---|---|---|
| Format entrée | texte brut → espeak → phonèmes | texte brut, interne G2P |
| Inférence | VITS ONNX, **Rust natif** (sonata-piper / ort) | non natif Rust ; torch/MeloTTS Python → ONNX export requis |
| Voix | ~20 FR/EN (siwis, alba, etc.) | limité, accent request-specific |
| Échantillonnage | 16 kHz | 16/24 kHz |
| Taille modèle | ~60–100 Mo | ~1–2 Go (dépendances exclues si ONNX seul) |
| RTF CPU | < 1 (similaire Kokoro) | ~1–2 s première synthèse |
| Intégration | **Se branche directement derrière `TtsEngine`** (espeak réutilisable, même contrat f32) | même contrat, mais chaîne G2P/horodatage plus lourde — accepté en dernier recours |
| Licence | MIT (Piper voices variées : vérifier) | MIT mais poids ~ Apache |

Verdict : le contrat `TtsEngine` tient pour les trois — Piper quasi tel quel (même posture « espeak + ONNX »), MeloTTS en passant par un export ONNX (coût d'ingénierie à réserver au cas Kokoro échoue latence/CPU/licence, aucun signal en ce sens à ce jour).

## Limites / à noter

1. `synthesize` est synchrone et CPU intensif : l'appelant (N-039/N-040/N-041) devra l'exécuter hors thread UI et vérifier le jeton de génération (N-020) entre segments — sinon l'arrêt pendant la synthèse est impossible.
2. `TtsLanguage::from_str_lenient` accepte « auto » comme inconnu → la détection FR/EN (N-036) viendra en amont ; le moteur ne gère pas « auto ».
3. Le trait n'expose ni preview sonore ni durée préditive : les moteurs qui le peuvent (Piper estime la durée) n'en profitent pas encore — volontairement hors contrat tant que la file audio (N-041) ne connaît pas la durée.
4. Voix FR `ff_siwis` CC-BY : redistribuée seulement si attribution embarquée (S0-08, TW-025) — non impactant pour le contrat.

## Traçabilité

- Carte : N-009. Sources : `src-tauri/src/tts/engine.rs`, `src-tauri/src/tts/mod.rs` (renommage `KokoroEngine`), `cargo test` (21 passed, dont 3 nouveaux).
- Critères de la carte : « Trait Rust défini » ✓ ; « utilisé par le prototype » ✓ (tests stub) ; « isolation pour remplacer le moteur » ✓ (Kokoro derrière le trait ; Piper/MeloTTS documentés).
- Le branchement effectif en production reste pour N-039 ; la file et l'arrêt coopératif pour N-040/N-041.
