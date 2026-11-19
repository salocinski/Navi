//! Segmentation en production (carte N-037) — texte nettoyé → file de segments.
//!
//! Stratégie héritée du prototype S0-10, durcie pour la production :
//! - découpage sur ponctuation forte `. ! ?` **et sauts de ligne** (frontière sûre) ;
//! - protection des abréviations FR/EN (`Dr.`, `M.`, `Mme`, `etc.`, `p.ex.`…) ;
//! - regroupement en chunks ≤ 480 tokens estimés (marge sous 510 chez Kokoro) ;
//! - segment isolé > limite → découpage aux frontières de mots ; un **mot** plus long que la
//!   limite est scindé en dur (le contrat `estimated_tokens ≤ max_tokens_per_chunk()` prime toujours) ;
//! - aucun `unicode-segmentation` : la ponctuation forte est ASCII, les compteurs en chars.

/// Limite d'inférence Kokoro 510 tokens → marge de sécurité.
/// Surchargeable via `NAVI_MAX_SEGMENT_TOKENS` (benchmarks, carte TW-040).
pub(crate) fn parse_max_tokens(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.parse::<usize>().ok())
        .filter(|&n| n >= 16 && n <= 510)
        .unwrap_or(480)
}

pub fn max_tokens_per_chunk() -> usize {
    parse_max_tokens(std::env::var("NAVI_MAX_SEGMENT_TOKENS").ok().as_deref())
}

/// Estimation grossière : 1 token ≈ 4 caractères (FR/EN). Documentée comme approximation ;
/// le comptage exact se fait à la tokenisation (N-039).
pub fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() as f64 / 4.0).ceil() as usize
}

/// Un segment prêt pour la file : texte + estimation de charge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub index: usize,
    pub text: String,
    pub estimated_tokens: usize,
}

/// Segmente un texte nettoyé (N-035) en chunks destinés à l'inférence Kokoro.
pub fn segment_text(text: &str) -> Vec<Segment> {
    let sentences = split_sentences(text);
    let chunks = chunk_by_token_limit(&sentences, max_tokens_per_chunk());
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, text)| Segment {
            index,
            estimated_tokens: estimate_tokens(&text),
            text,
        })
        .collect()
}

/// Découpe sur ponctuation forte et sauts de ligne.
///
/// Règle : un `.`, `!` ou `?` termine une phrase **uniquement** s'il est suivi d'un espace
/// ou de la fin du texte — sauf abréviation courte (voir `is_abbreviation`).
pub fn split_sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut current = String::new();

    for (i, g) in chars.iter().enumerate() {
        current.push(*g);

        // Frontière par saut de ligne
        if *g == '\n' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
            continue;
        }

        let is_sentence_end =
            matches!(g, '.' | '!' | '?') && (i + 1 == chars.len() || chars[i + 1].is_whitespace());

        if is_sentence_end && !is_abbreviation(&current) {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }
    sentences
}

/// Heuristique d'abréviation : dernier mot court terminé par `.` et alphabétique
/// ou faisant partie d'une liste FR/EN (`M.`, `Dr.`, `Mme`, `etc.`, `p.ex.`…).
fn is_abbreviation(current: &str) -> bool {
    const ABBR_FR: &[&str] = &[
        "M.", "Mme", "Mme.", "Mlle.", "Mlle", "Dr.", "St.", "etc.", "p.ex.", "c.-à-d.", "ndlr.",
        "Mgr.",
    ];
    const ABBR_EN: &[&str] = &[
        "Mr.", "Mrs.", "Ms.", "Dr.", "St.", "Mt.", "etc.", "e.g.", "i.e.", "vs.", "Inc.", "Ltd.",
    ];

    let last_word = match current.split_whitespace().next_back() {
        Some(w) => w,
        None => return false,
    };

    if ABBR_FR.contains(&last_word) || ABBR_EN.contains(&last_word) {
        return true;
    }

    // Initiale : une seule lettre majuscule + '.' (ex. « J. R. R. Tolkien »)
    let mut chars = last_word.chars();
    matches!(chars.next(), Some(c) if c.is_uppercase())
        && last_word.len() == 2
        && last_word.ends_with('.')
}

/// Regroupe les phrases en chunks tenant dans `max_tokens` (estimation 1 token ≈ 4 chars).
pub fn chunk_by_token_limit(sentences: &[String], max_tokens: usize) -> Vec<String> {
    let max_chars = max_tokens * 4;
    let mut chunks = Vec::new();
    let mut current = String::new();

    for s in sentences {
        let candidate = if current.is_empty() {
            s.clone()
        } else {
            format!("{current} {s}")
        };

        if candidate.chars().count() <= max_chars {
            current = candidate;
            continue;
        }

        if !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }

        if s.chars().count() > max_chars {
            chunks.extend(chunk_plain_text(s, max_chars));
        } else {
            current = s.to_string();
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Un segment isolé trop long → découpage aux frontières de mots.
/// Un **mot** plus long que la limite est découpé en dur (jamais de chunk > max_chars).
pub fn chunk_plain_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if candidate.chars().count() <= max_chars {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        // Un mot plus long que la limite : découpage dur en tranches de max_chars
        let mut rest = word;
        while rest.chars().count() > max_chars {
            let (head, tail) = split_at_char_count(rest, max_chars);
            chunks.push(head.to_string());
            rest = tail;
        }
        current = rest.to_string();
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn split_at_char_count(s: &str, count: usize) -> (&str, &str) {
    match s.char_indices().nth(count) {
        Some((idx, _)) => s.split_at(idx),
        None => (s, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_simples() {
        let segs = split_sentences("Un. Deux. Trois !");
        assert_eq!(segs, vec!["Un.", "Deux.", "Trois !"]);
    }

    #[test]
    fn abreviation_protegee() {
        let segs = split_sentences("Dr. Martin habite ici. C'est vrai !");
        assert_eq!(segs, vec!["Dr. Martin habite ici.", "C'est vrai !"]);
        let segs = split_sentences("etc. C'est tout.");
        assert_eq!(segs, vec!["etc. C'est tout."]);
        let segs = split_sentences("M. Dupont. Il part.");
        assert_eq!(segs, vec!["M. Dupont.", "Il part."]);
    }

    #[test]
    fn sauts_de_ligne_sont_frontieres() {
        let segs = split_sentences("ligne un\nligne deux\nligne trois");
        assert_eq!(segs, vec!["ligne un", "ligne deux", "ligne trois"]);
    }

    #[test]
    fn estimate_tokens_arrondi() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn max_tokens_env_override() {
        assert_eq!(parse_max_tokens(None), 480);
        assert_eq!(parse_max_tokens(Some("64")), 64);
        assert_eq!(parse_max_tokens(Some("3")), 480); // sous le plancher : refusé
        assert_eq!(parse_max_tokens(Some("511")), 480); // au-dessus de 510 : refusé
        assert_eq!(parse_max_tokens(Some("abc")), 480);
        assert_eq!(parse_max_tokens(Some("")), 480);
    }

    #[test]
    fn chunk_limit_respectee() {
        let s = "a".repeat(2500); // ~625 tokens, sans ponctuation
        let chunks = segment_text(&s);
        assert!(chunks.len() >= 2);
        for seg in &chunks {
            assert!(
                seg.estimated_tokens <= max_tokens_per_chunk() + 1,
                "segment trop gros : {} tokens",
                seg.estimated_tokens
            );
        }
    }

    #[test]
    fn segment_long_coupe_aux_frontieres_de_mots() {
        let word = "a".repeat(100);
        let parts = chunk_plain_text(&word, 480 * 4);
        // Un mot > limite est conservé entier (jamais cassé)
        assert_eq!(parts, vec![word]);
    }

    #[test]
    fn texte_long_en_deux_chunks() {
        // ~35 phrases → doit tenir en 2 chunks (comme le prototype S0)
        let phrase = "Voici un texte beaucoup plus long pour tester la segmentation en chunks. ";
        let long_text = phrase.repeat(35);
        let segs = segment_text(&long_text);
        assert_eq!(segs.len(), 2);
        assert!(segs[0].estimated_tokens + segs[1].estimated_tokens < 800);
    }

    #[test]
    fn chunk_ordre_et_index_stables() {
        let segs = segment_text("A. B. C. D. E. F. G. H.");
        for (i, seg) in segs.iter().enumerate() {
            assert_eq!(seg.index, i);
        }
    }

    #[test]
    fn texte_vide_donne_aucun_segment() {
        assert!(segment_text("").is_empty());
        assert!(segment_text("   \n  ").is_empty());
    }

    #[test]
    fn regroupement_conserve_le_texte() {
        let input = "Un. Deux. Trois. Quatre.";
        let segs = segment_text(input);
        let joined: Vec<&str> = segs.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined.join(" "), input);
    }
}
