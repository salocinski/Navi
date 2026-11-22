//! Pipeline de lecture — texte → segments → Kokoro → file audio (carte N-039).
//!
//! L'objectif de cette carte : brancher Kokoro derrière `TtsEngine` avec un
//! chargement en tâche de fond et alimenter la file audio en segments synthétisés,
//! avec annulation coopérative via le jeton de génération (N-020).
//!
//! Structure :
//! - `AudioSink` : sortie injectable — `SharedAudioSink` en production (device requis),
//!   mémoire en test.
//! - `synthesize_segments` : boucle pure (moteur → sink) avec vérification du jeton
//!   entre chaque segment ; testable sans périphérique.
//! - Le câblage Tauri (commandes, états, events) vit dans `lib.rs`.

#[cfg(test)]
use std::sync::{Arc, Mutex};

use crate::text::segmentation::Segment;
use crate::tts::SynthesisRequest;
use crate::tts::engine::MAX_TOKENS_PER_SEGMENT;
use crate::tts::engine::TtsEngine;
use crate::tts::engine::TtsLanguage;

/// Sortie audio injectable.
pub trait AudioSink: Send + Sync {
    /// Pousse les échantillons dans la file audio.
    fn write(&self, samples: &[f32]) -> Result<(), String>;
}

// Sortie production : `crate::audio::AudioHandle` implémente AudioSink
// (envoi `Enqueue` par canal mpsc vers le thread audio) — aucun périphérique requis côté écrivain.

/// Sortie mémoire pour les tests (aucun device requis).
#[cfg(test)]
pub struct MemorySink {
    pub chunks: Arc<Mutex<Vec<Vec<f32>>>>,
}

#[cfg(test)]
impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl MemorySink {
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[cfg(test)]
impl AudioSink for MemorySink {
    fn write(&self, samples: &[f32]) -> Result<(), String> {
        self.chunks
            .lock()
            .map_err(|_| "verrou".to_string())?
            .push(samples.to_vec());
        Ok(())
    }
}

/// Recoupe `text` jusqu'à ce que chaque morceau tienne sous `MAX_TOKENS_PER_SEGMENT`
/// **tokens réels** (carte N-084). La segmentation amont n'est qu'une estimation : les nombres
/// et abréviations s'allongent à la phonémisation. Coupe à l'espace le plus proche du milieu,
/// à défaut au milieu du mot ; un texte d'un seul caractère n'est jamais recoupé.
pub(crate) fn fit_segments(
    engine: &dyn TtsEngine,
    text: &str,
    language: Option<TtsLanguage>,
) -> Result<Vec<String>, String> {
    let tokens = engine
        .count_tokens(text, language)
        .map_err(|e| format!("Comptage des tokens impossible : {e}"))?;
    let chars: Vec<char> = text.chars().collect();
    if tokens <= MAX_TOKENS_PER_SEGMENT || chars.len() < 2 {
        return Ok(vec![text.to_string()]);
    }
    let mid = chars.len() / 2;
    let cut = (1..chars.len())
        .filter(|&i| chars[i].is_whitespace())
        .min_by_key(|&i| i.abs_diff(mid))
        .unwrap_or(mid);
    let (left, right): (String, String) =
        (chars[..cut].iter().collect(), chars[cut..].iter().collect());
    let mut out = Vec::new();
    for part in [left, right] {
        if !part.trim().is_empty() {
            out.extend(fit_segments(engine, part.trim(), language)?);
        }
    }
    Ok(out)
}

/// Synthétise chaque segment et pousse l'audio dans `sink`.
///
/// S'arrête avant un segment si `is_cancelled()` retourne vrai — l'appelant
/// (`stop`) vide ensuite la file audio et invalide le jeton (TW-020).
/// Retourne le nombre de segments synthétisés.
pub fn synthesize_segments(
    engine: &dyn TtsEngine,
    segments: &[Segment],
    sink: &dyn AudioSink,
    is_cancelled: &dyn Fn() -> bool,
    language: Option<TtsLanguage>,
    on_segment_done: &dyn Fn(),
) -> Result<usize, String> {
    let mut count = 0;
    for seg in segments {
        if is_cancelled() {
            eprintln!(
                "[navi] synthèse annulée après {count}/{} segments",
                segments.len()
            );
            return Ok(count);
        }
        eprintln!(
            "[navi] synthèse segment {}/{} ({} chars)…",
            seg.index + 1,
            segments.len(),
            seg.text.chars().count()
        );
        let mut samples = 0;
        for piece in fit_segments(engine, &seg.text, language)? {
            let out = engine
                .synthesize(&SynthesisRequest {
                    text: piece,
                    voice: None,
                    language,
                    speed: 1.0,
                })
                .map_err(|e| {
                    format!(
                        "Synthèse du segment {}/{} impossible : {e}",
                        seg.index + 1,
                        segments.len()
                    )
                })?;
            sink.write(&out.samples)?;
            samples += out.samples.len();
        }
        count += 1;
        on_segment_done();
        eprintln!(
            "[navi] segment {}/{} synthétisé ({} échantillons)",
            count,
            segments.len(),
            samples
        );
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::engine::TtsLanguage;

    struct StubEngine {
        loaded: bool,
    }

    impl TtsEngine for StubEngine {
        fn name(&self) -> &'static str {
            "stub"
        }
        fn supported_languages(&self) -> &'static [TtsLanguage] {
            &[TtsLanguage::French, TtsLanguage::English]
        }
        fn voices(&self) -> Vec<String> {
            vec!["stub".to_string()]
        }
        fn load(&mut self) -> Result<(), crate::tts::TtsError> {
            Ok(())
        }
        fn is_loaded(&self) -> bool {
            true
        }
        fn synthesize(
            &self,
            request: &SynthesisRequest,
        ) -> Result<crate::tts::SynthesisOutput, crate::tts::TtsError> {
            if !self.loaded {
                return Err(crate::tts::TtsError::NotLoaded);
            }
            Ok(crate::tts::SynthesisOutput {
                samples: vec![0.5; request.text.chars().count()],
                sample_rate: 24_000,
            })
        }
    }

    fn segments(n: usize) -> Vec<Segment> {
        (0..n)
            .map(|i| Segment {
                index: i,
                text: format!("segment {i}"),
                estimated_tokens: 10,
            })
            .collect()
    }

    #[test]
    fn synthetise_tous_les_segments_dans_l_ordre() {
        let sink = MemorySink::new();
        let engine = StubEngine { loaded: true };
        let n = synthesize_segments(
            &engine,
            &segments(5),
            &sink,
            &|| false,
            Some(TtsLanguage::French),
            &|| (),
        )
        .unwrap();
        assert_eq!(n, 5);
        let chunks = sink.chunks.lock().unwrap();
        assert_eq!(chunks.len(), 5);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.len(), format!("segment {i}").chars().count());
        }
    }

    #[test]
    fn l_annulation_arrete_avant_le_segment_suivant() {
        let sink = MemorySink::new();
        let engine = StubEngine { loaded: true };
        let count = std::cell::Cell::new(0usize);
        let is_cancelled = || {
            count.set(count.get() + 1);
            count.get() > 2 // après 2 segments, on annule
        };
        let n = synthesize_segments(
            &engine,
            &segments(5),
            &sink,
            &is_cancelled,
            Some(TtsLanguage::French),
            &|| (),
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(sink.chunks.lock().unwrap().len(), 2);
    }

    /// Moteur dont le comptage réel vaut 2 tokens par caractère (phonémisation qui s'allonge).
    struct DoubleCount;
    impl TtsEngine for DoubleCount {
        fn name(&self) -> &'static str {
            "double"
        }
        fn supported_languages(&self) -> &'static [TtsLanguage] {
            &[TtsLanguage::French]
        }
        fn voices(&self) -> Vec<String> {
            vec![]
        }
        fn load(&mut self) -> Result<(), crate::tts::TtsError> {
            Ok(())
        }
        fn is_loaded(&self) -> bool {
            true
        }
        fn synthesize(
            &self,
            request: &SynthesisRequest,
        ) -> Result<crate::tts::SynthesisOutput, crate::tts::TtsError> {
            Ok(crate::tts::SynthesisOutput {
                samples: vec![0.0; request.text.chars().count()],
                sample_rate: 24_000,
            })
        }
        fn count_tokens(
            &self,
            text: &str,
            _l: Option<TtsLanguage>,
        ) -> Result<usize, crate::tts::TtsError> {
            Ok(text.chars().count() * 2)
        }
    }

    #[test]
    fn fit_segments_laisse_un_texte_court_intact() {
        let v = fit_segments(&DoubleCount, "bonjour le monde", None).unwrap();
        assert_eq!(v, vec!["bonjour le monde"]);
    }

    #[test]
    fn fit_segments_recoupe_sur_les_tokens_reels() {
        let text = vec!["mot"; 300].join(" "); // 1199 chars → 2398 tokens réels
        let v = fit_segments(&DoubleCount, &text, None).unwrap();
        assert!(v.len() >= 5);
        for p in &v {
            assert!(
                p.chars().count() * 2 <= MAX_TOKENS_PER_SEGMENT,
                "trop long : {}",
                p.len()
            );
        }
        assert_eq!(v.join(" "), text, "aucun mot perdu ni coupé");
    }

    #[test]
    fn fit_segments_scinde_un_mot_sans_espace() {
        let text = "a".repeat(600);
        let v = fit_segments(&DoubleCount, &text, None).unwrap();
        assert!(
            v.iter()
                .all(|p| p.chars().count() * 2 <= MAX_TOKENS_PER_SEGMENT)
        );
        assert_eq!(v.concat(), text);
    }

    #[test]
    fn la_langue_est_transmise() {
        let sink = MemorySink::new();
        struct LangProbe {
            seen: Arc<Mutex<Option<TtsLanguage>>>,
        }
        impl TtsEngine for LangProbe {
            fn name(&self) -> &'static str {
                "probe"
            }
            fn supported_languages(&self) -> &'static [TtsLanguage] {
                &[TtsLanguage::French, TtsLanguage::English]
            }
            fn voices(&self) -> Vec<String> {
                vec![]
            }
            fn load(&mut self) -> Result<(), crate::tts::TtsError> {
                Ok(())
            }
            fn is_loaded(&self) -> bool {
                true
            }
            fn synthesize(
                &self,
                request: &SynthesisRequest,
            ) -> Result<crate::tts::SynthesisOutput, crate::tts::TtsError> {
                *self.seen.lock().unwrap() = request.language;
                Ok(crate::tts::SynthesisOutput {
                    samples: vec![0.0],
                    sample_rate: 24_000,
                })
            }
        }
        let seen = Arc::new(Mutex::new(None));
        let probe = LangProbe { seen: seen.clone() };
        let segs = segments(1);
        synthesize_segments(
            &probe,
            &segs,
            &sink,
            &|| false,
            Some(TtsLanguage::English),
            &|| (),
        )
        .unwrap();
        assert_eq!(*seen.lock().unwrap(), Some(TtsLanguage::English));
    }
}
