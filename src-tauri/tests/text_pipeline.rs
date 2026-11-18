//! Corpus d'intégration pour le pipeline texte → segments prêt-kokoro (carte N-038).
//!
//! Tests de niveau crate : exercés sur l'API publique de `navi_lib`
//! (aucun périphérique audio, aucune inférence ONNX).

use navi_lib::text::{
    clean_captured_text, detect_language, html_to_text, rtf_to_text, strip_urls, LanguageDetection,
};
use navi_lib::text::segmentation::segment_text;

// ---------- Corpus Word/Chromium (fragments réels) ----------

#[test]
fn corpus_fragment_word_tableau() {
    // Fragment Word-style avec tableau
    let html = r#"<!--StartFragment--><table border="1"><tr><td>Nom</td><td>Valeur</td></tr>
    <tr><td>Éléphant</td><td>Grand mammifère</td></tr></table><!--EndFragment-->"#;
    let cleaned = clean_captured_text(html, "html").unwrap();
    assert!(cleaned.contains("Nom"));
    assert!(cleaned.contains("Éléphant"));
    assert!(cleaned.contains("Grand mammifère"));
}

#[test]
fn corpus_fragment_chromium() {
    let html = r#"<!--StartFragment--><div>Candidat <b>solide</b></div><!--EndFragment-->"#;
    let cleaned = clean_captured_text(html, "html").unwrap();
    assert!(cleaned.contains("Candidat"));
    assert!(cleaned.contains("solide"));
    assert!(!cleaned.contains('<'));
}

#[test]
fn corpus_chromium_with_internal_comments() {
    // Chromium insère des commentaires internes dans ses fragments
    let html = r#"<!--StartFragment--><!--s--><span>Texte réel ici</span><!--EndFragment-->"#;
    let cleaned = clean_captured_text(html, "html").unwrap();
    assert!(cleaned.contains("Texte réel ici"));
}

// ---------- Pipeline complet sur paragraphe FR ----------

fn para_fr() -> String {
    "Bonjour. Je suis Navi, un assistant local. Dr. Martin habite au 12 rue de la Paix. ".repeat(30)
}

#[test]
fn corpus_paragraphe_fr_pipeline_complet() {
    // nettoyage (accents de la phrase de test)
    let cleaned = clean_captured_text(&para_fr(), "unicode").unwrap();
    // "Dr." est développé en "Docteur" par l'expansion d'abréviations FR (souhaitée pour espeak)
    assert!(cleaned.contains("Docteur Martin"));
    // détection
    assert_eq!(detect_language(&cleaned), LanguageDetection::French);
    // segmentation
    let segs = segment_text(&cleaned);
    assert!(segs.len() >= 2);
    for seg in &segs {
        assert!(seg.estimated_tokens <= 480);
        assert!(!seg.text.is_empty());
    }
    // reproduction du texte (modulo espaces multiples)
    let joined: String = segs.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
    assert!(joined.contains("Docteur Martin"));
}

fn para_en() -> String {
    "Hello. I am Navi, a local assistant. Dr. Smith lives at 12 Main Street. ".repeat(30)
}

#[test]
fn corpus_paragraphe_en_pipeline_complet() {
    let cleaned = clean_captured_text(&para_en(), "unicode").unwrap();
    assert_eq!(detect_language(&cleaned), LanguageDetection::English);
    let segs = segment_text(&cleaned);
    assert!(segs.len() >= 2);
    for seg in &segs {
        assert!(seg.estimated_tokens <= 480);
    }
}

#[test]
fn corpus_texte_mixte_detecte_mixed() {
    let mixed = "C'est un excellent code, the implementation is clean and it works very well";
    assert_eq!(detect_language(mixed), LanguageDetection::Mixed);
}

// ---------- CRLF / cas Windows ----------

#[test]
fn corpus_crlf_normalise() {
    let raw = "Ligne un\r\nLigne deux\r\nLigne trois\r\n";
    let cleaned = clean_captured_text(raw, "unicode").unwrap();
    assert!(cleaned.contains("Ligne un"));
    assert!(cleaned.contains("Ligne deux"));
    // pas de \r résiduel
    assert!(!cleaned.contains('\r'));
}

// ---------- RTF edge cases ----------

#[test]
fn corpus_rtf_bold_et_italique() {
    let rtf = r"{\rtf1\ansi Texte \b gras \b0 et \i italique\i0 . Fin}";
    let out = rtf_to_text(rtf);
    assert!(out.contains("Texte"));
    assert!(out.contains("gras"));
    assert!(out.contains("italique"));
    assert!(!out.contains("\\b"));
}

// ---------- URLs ----------

#[test]
fn corpus_urls_dans_prose() {
    let raw = "Voir la doc sur https://example.com/page pour plus d'informations ensuite.";
    let out = strip_urls(raw);
    assert_eq!(out, "Voir la doc sur pour plus d'informations ensuite.");
}

// ---------- Propriétés structurelles (déterministe, sans proptest) ----------

#[test]
fn corpus_segments_toujours_bornes() {
    // 100 phrases déterministes — chaque groupe doit respecter 480 tokens
    let mut corpus = String::new();
    for i in 0..100 {
        corpus.push_str(&format!("Phrase numéro {i} avec du texte de test non négligeable. "));
    }
    let segs = segment_text(&corpus);
    assert!(segs.len() >= 2);
    let mut total_chars = 0usize;
    for seg in &segs {
        assert!(seg.estimated_tokens <= 480);
        total_chars += seg.text.chars().count();
    }
    assert!(total_chars > 1000);
}

#[test]
fn segments_ancres_dans_le_texte_continu() {
    // Aucun morceau ne doit être perdu : la somme des mots des segments == somme des mots de l'entrée
    let input = "Un. Deux. Trois. Quatre. Cinq. Six.";
    let segs = segment_text(input);
    let joined: Vec<String> = segs.iter().map(|s| s.text.clone()).collect();
    assert_eq!(joined.join(" "), input);
}

// ---------- Limites assertées comportement actuel ----------

#[test]
fn corpus_limite_html_mal_forme_no_panic() {
    // Impacté : l'attribut contenant '>' casse le strip générique → comportement partiel, jamais de panic
    let html = "<a title=\"a > b\">Lien</a> suite";
    let out = html_to_text(html);
    assert!(out.contains("Lien"));
    assert!(out.contains("suite"));
}

#[test]
fn corpus_html_bloc_non_fermes_no_panic() {
    // balise bloquante jamais fermée → tout le reste ignoré (comportement documenté), pas de panic
    let html = "Texte avant <script>var x = 1;";
    let out = html_to_text(html);
    assert!(out.contains("Texte avant"));
}

#[test]
fn corpus_mot_unique_superieur_a_la_limite() {
    // Mot contigu > 1920 chars → scindé en dur (contrat 480 tokens prime)
    let word = "a".repeat(5000);
    let segs = segment_text(&word);
    assert!(segs.len() >= 3);
    for seg in &segs {
        assert!(seg.estimated_tokens <= 481);
    }
}

#[test]
fn corpus_clean_reel_documents() {
    // Provoqué : HTML orpheline → résultat vide/erreur claire
    let html = "<img src=\"data:image/png;base64,AAAA\">";
    let err = clean_captured_text(html, "html").unwrap_err();
    assert!(err.to_string().contains("Aucun texte exploitable"));
}
