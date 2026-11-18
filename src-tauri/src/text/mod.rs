//! Nettoyage et normalisation du texte capturé (carte N-035).
//!
//! Pipeline : détection de fragment HTML réel → suppression markup/médias →
//! décodage entités → URLs/data URIs supprimés → normalisation espaces/newlines
//! → abréviations FR de base → texte prêt pour la phonémisation (espeak).
//!
//! Exigence tracée (revue TW-014) : une sélection sans texte exploitable
//! renvoie `CleanError::NoText` — jamais du markup lu à voix haute.

use std::fmt;

/// Erreurs de nettoyage — message FR clair pour l'UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanError {
    /// Rien d'exploitable après nettoyage (markup seul, médias seuls, texte vide).
    NoText,
}

impl fmt::Display for CleanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleanError::NoText => write!(
                f,
                "Aucun texte exploitable dans la sélection — le contenu est vide, composé de médias (images/vidéos) ou de markup seul"
            ),
        }
    }
}

impl std::error::Error for CleanError {}

/// Nettoie un texte capturé selon le format d'origine.
pub fn clean_captured_text(raw: &str, format_hint: &str) -> Result<String, CleanError> {
    let cleaned = match format_hint.to_ascii_lowercase().as_str() {
        "html" => html_to_text(raw),
        "rtf" => rtf_to_text(raw),
        _ => normalize_ws(raw),
    };

    let cleaned = strip_urls(&cleaned);
    let cleaned = expand_abbrev_fr(&cleaned);
    let cleaned = normalize_ws(&cleaned);

    if cleaned.trim().is_empty() {
        return Err(CleanError::NoText);
    }
    Ok(cleaned)
}

/// Normalisation générique : espaces, newlines, caractères de contrôle.
/// (Conservée pour la compatibilité avec `utils::clean_text`.)
pub fn normalize_ws(text: &str) -> String {
    // 1) Espaces insécables/fins/typo et NBSP → espace ASCII ; contrôle (hors \n \t) → supprimé
    let mut buf = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch as u32 {
            0x00A0 | 0x202F | 0x2007 | 0x2060 => buf.push(' '),
            0x0009..=0x000A | 0x000D..=0x000D => buf.push(if ch == '\t' { ' ' } else { '\n' }),
            c if c < 0x0020 || (0x007F..0x00A0).contains(&c) => {} // contrôles supprimés
            _ => buf.push(ch),
        }
    }

    // 2) Collapse [ \t]+ → un espace, trim lignes, collapse 3+ newlines → 2
    let mut lines: Vec<String> = Vec::new();
    let mut blank_run = 0usize;
    for line in buf.lines() {
        let collapsed: String = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                lines.push(String::new());
            }
        } else {
            blank_run = 0;
            lines.push(collapsed);
        }
    }

    // 3) Trim leading/trailing newlines
    let joined = lines.join("\n");
    joined.trim_matches('\n').to_string()
}

/// HTML → texte : garde uniquement le fragment réel, supprime médias/markup.
///
/// Gère : en-têtes clipboard (StartFragment…), `<script>/<style>/<svg>/<noscript>`,
/// `<img>/<video>/<audio>/<object>/<embed>/<canvas>` (éléments et contenu bortés),
/// balises `meta/link/br/hr/input`, entités (`&amp;`, `&#39;`, …), URLs et `data:`.
pub fn html_to_text(html: &str) -> String {
    // 1) Extraire le fragment réel s'il est délimité (sinon tout)
    let fragment = extract_fragment(html);

    // 2) Supprimer commentaires <!-- … -->
    let no_comments = strip_delimited(&fragment, "<!--", "-->");

    // 3) Supprimer éléments à contenu bloqué (script/style/svg/noscript/iframe/object/embed/video/audio/canvas/applet)
    let no_blocked = strip_blocked_elements(&no_comments);

    // 4) Supprimer toutes les balises restantes (void et ordinaires), garder le texte
    let no_tags = strip_tags(&no_blocked);

    // 5) Décoder les entités HTML
    decode_entities(&no_tags)
}

fn extract_fragment(html: &str) -> String {
    const START: &str = "<!--StartFragment-->";
    const END: &str = "<!--EndFragment-->";
    let lower = html.to_lowercase();
    let start = match lower.find(&START.to_lowercase()) {
        Some(i) => i + START.len(),
        None => return html.to_string(),
    };
    let end = lower[start..].find(&END.to_lowercase()).map(|i| start + i).unwrap_or(html.len());
    html[start..end].to_string()
}

fn strip_delimited(input: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        match rest.find(open) {
            Some(start) => {
                out.push_str(&rest[..start]);
                match rest[start..].find(close) {
                    Some(rel) => rest = &rest[start + rel + close.len()..],
                    None => break, // commentaire jamais fermé → ignorer la fin
                }
            }
            None => { out.push_str(rest); break; }
        }
    }
    out
}

/// Supprime les éléments dont le contenu est bloqué (script/style/svg…) :
/// balance simple — on cherche le `</name>` correspondant ; si absent, tout le reste est ignoré.
fn strip_blocked_elements(input: &str) -> String {
    const BLOCKED: &[&str] = &[
        "script", "style", "svg", "noscript", "iframe", "object", "embed", "video", "audio", "canvas", "applet",
    ];
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    'outer: while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let after = &rest[lt..];
        // balise de fin orpheline ? on la conserve textuellement pour le stripper de balises
        if after.starts_with("</") {
            out.push('<');
            rest = &after[1..];
            continue 'outer;
        }
        // lire le nom de la balise
        let mut name_end = 1;
        for (i, ch) in after.char_indices().skip(1) {
            if ch.is_ascii_alphanumeric() {
                name_end = i + 1;
            } else {
                break;
            }
        }
        let name = after[1..name_end].to_lowercase();
        if BLOCKED.contains(&name.as_str()) {
            let close_pat = format!("</{name}");
            let lower = after.to_lowercase();
            match lower.find(&close_pat) {
                Some(pos) => {
                    // sauter jusqu'à la fin de la balise fermante
                    let close_start = pos;
                    match after[close_start..].find('>') {
                        Some(rel) => rest = &after[close_start + rel + 1..],
                        None => break,
                    }
                }
                None => break, // élément jamais fermé → ignorer tout le reste
            }
        } else {
            // pas un élément bloqué : laisser la balise pour le stripper générique
            out.push('<');
            rest = &after[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Supprime toutes les balises ; les balises de bloc deviennent des sauts de ligne.
fn strip_tags(input: &str) -> String {
    const BLOCK_TAGS: &[&str] = &[
        "p", "div", "br", "hr", "li", "ul", "ol", "tr", "td", "th", "table", "h1", "h2", "h3", "h4", "h5", "h6",
        "section", "article", "header", "footer", "blockquote", "pre", "figure", "figcaption", "dd", "dt",
    ];
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        match rest[lt..].find('>') {
            Some(rel) => {
                let tag_src = &rest[lt + 1..lt + rel];
                let name: String = tag_src
                    .trim_start_matches('/')
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_lowercase();
                // <br> et autres blocs → newline ; fermeture </p> etc. → newline également
                if BLOCK_TAGS.contains(&name.as_str()) {
                    out.push('\n');
                }
                rest = &rest[lt + rel + 1..];
            }
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        match rest.find('&') {
            Some(amp) => {
                out.push_str(&rest[..amp]);
                let after = &rest[amp..];
                match after.find(';') {
                    Some(semi) => {
                        let entity = &after[1..semi];
                        match decode_entity(entity) {
                            Some(decoded) => out.push_str(&decoded),
                            None => out.push_str(&format!("&{entity};")),
                        }
                        rest = &after[semi + 1..];
                    }
                    None => {
                        out.push('&');
                        rest = &after[1..];
                    }
                }
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

fn decode_entity(entity: &str) -> Option<String> {
    let named = match entity {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        "nbsp" => Some(" "),
        _ => None,
    };
    if let Some(n) = named {
        return Some(n.to_string());
    }
    let numeric = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse::<u32>().ok()));
    numeric.and_then(char::from_u32).map(|c| c.to_string())
}

/// Supprime URLs nues et `data:` URIs (le contenu n'est pas lisible) — ligne par ligne,
/// en conservant la structure des sauts de ligne.
pub fn strip_urls(text: &str) -> String {
    fn is_url_token(token: &str) -> bool {
        let lower = token.to_lowercase();
        lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("www.")
            || lower.starts_with("data:")
            || lower.starts_with("ftp://")
            || lower.starts_with("mailto:")
    }

    text.lines()
        .map(|line| line.split_whitespace().filter(|t| !is_url_token(t)).collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// RTF → texte (meilleur effort ; RTF complexe → texte approximatif).
pub fn rtf_to_text(rtf: &str) -> String {
    // Supprimer les groupes de tables (métadonnées)
    let mut s = rtf.to_string();
    for grp in ["fonttbl", "colortbl", "stylesheet", "info", "*"] {
        loop {
            let pat = format!("{{\\{grp}");
            match s.find(&pat) {
                Some(start) => {
                    let bytes = s.as_bytes();
                    let mut depth = 0usize;
                    let mut end = start;
                    for (i, b) in bytes.iter().enumerate().skip(start) {
                        match b {
                            b'{' => depth += 1,
                            b'}' => {
                                depth -= 1;
                                if depth == 0 { end = i + 1; break; }
                            }
                            _ => {}
                        }
                    }
                    s.replace_range(start..end, "");
                }
                None => break,
            }
        }
    }
    // \par / \line / \br → saut de ligne
    for seq in ["\\par", "\\line", "\\br"] {
        s = s.replace(seq, "\n");
    }
    // Supprimer les mots de contrôle \word[digits][space]
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(bs) = rest.find('\\') {
        out.push_str(&rest[..bs]);
        let after = &rest[bs + 1..];
        let mut i = 0;
        for ch in after.char_indices() {
            if ch.1.is_ascii_alphabetic() || ch.1 == '*' { i = ch.0 + 1; continue; }
            if ch.1.is_ascii_digit() { i = ch.0 + 1; continue; }
            if ch.1 == ' ' { i = ch.0 + 1; }
            break;
        }
        rest = &after[i..];
    }
    out.push_str(rest);
    out.replace(['{', '}'], "")
}

/// Abréviations FR courantes — expansion minimale pour un meilleur rendu espeak.
fn expand_abbrev_fr(text: &str) -> String {
    const MAP: &[(&str, &str)] = &[
        ("M.", "Monsieur"),
        ("Mme", "Madame"),
        ("Mlle", "Mademoiselle"),
        ("Dr", "Docteur"),
        ("Mme.", "Madame"),
        ("p.ex.", "par exemple"),
        ("cf.", "confer"),
        ("c.-à-d.", "c'est-à-dire"),
        ("ndlr.", "note de la rédaction"),
    ];
    let mut out = text.to_string();
    for (abbr, full) in MAP {
        out = replace_word(&out, abbr, full);
    }
    out
}

fn replace_word(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find(from) {
        let before = if pos == 0 { ' ' } else { rest[..pos].chars().next_back().unwrap() };
        let after_idx = pos + from.len();
        let after = if after_idx >= rest.len() { ' ' } else { rest[after_idx..].chars().next().unwrap() };
        let boundary_before = !before.is_alphanumeric() && before != '_';
        let boundary_after = !after.is_alphanumeric() && after != '_';
        if boundary_before && boundary_after {
            out.push_str(&rest[..pos]);
            out.push_str(to);
            rest = &rest[after_idx..];
        } else {
            out.push_str(&rest[..pos + from.len()]);
            rest = &rest[pos + from.len()..];
        }
    }
    out.push_str(rest);
    out
}

mod language {
    //! Détection FR/EN indépendante du moteur TTS (carte N-036).

    /// Résultat de détection — jamais de panic, jamais « au hasard ».
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LanguageDetection {
        French,
        English,
        /// Les deux langues sont représentées (aucun gagnant clair) — documenter le comportement (voir `as_tts_hint`).
        Mixed,
        /// Texte trop court ou sans indice — même politique que `Mixed`.
        Unknown,
    }

    impl LanguageDetection {
        /// Libellé pour l'UI (« fr »/« en »/« mixed »/« unknown »).
        pub fn label(&self) -> &'static str {
            match self {
                LanguageDetection::French => "fr",
                LanguageDetection::English => "en",
                LanguageDetection::Mixed => "mixed",
                LanguageDetection::Unknown => "unknown",
            }
        }

        /// Hint pour Kokoro/phonémisation : en cas de doute (Mixed/Unknown),
        /// décision produit = **fr par défaut** (cible produit) — l'utilisateur
        /// pourra forcer la langue via N-053. Documenté dans `docs/s1-36-...`.
        pub fn as_tts_hint(&self) -> &'static str {
            match self {
                LanguageDetection::French | LanguageDetection::Mixed | LanguageDetection::Unknown => "fr",
                LanguageDetection::English => "en",
            }
        }
    }

    const FR_STOPWORDS: &[&str] = &[
        "le", "la", "les", "un", "une", "des", "est", "dans", "que", "qui", "pas", "avec", "pour", "sur",
        "aux", "du", "ce", "cette", "ou", "mais", "donc", "car", "sous", "sont", "était", "nous", "vous",
        "je", "tu", "il", "elle", "ils", "elles", "à", "au", "se", "son", "sa", "ses", "leur", "même",
        "très", "être", "avoir", "sans", "chaque", "entre", "aussi", "peut", "peut-être", "avant", "après",
        "alors", "encore", "ici", "là", "jamais", "toujours", "beaucoup", "votre", "notre", "son", "autre",
        "autrement", "fait", "comme", "dès", "quand",
    ];

    const EN_STOPWORDS: &[&str] = &[
        "the", "of", "and", "is", "in", "to", "for", "with", "that", "this", "it", "are", "was", "were",
        "be", "been", "have", "has", "had", "not", "you", "we", "they", "he", "she", "its", "as", "on",
        "at", "by", "an", "or", "from", "but", "not", "if", "then", "than", "so", "do", "does", "did",
        "done", "their", "there", "here", "when", "what", "where", "which", "who", "how", "each", "other",
        "than", "while", "about", "just", "into", "over", "under", "after", "before", "between",
    ];

    const FR_ACCENTS: &[char] = &[
        'é', 'è', 'ê', 'ë', 'à', 'â', 'ä', 'ç', 'î', 'ï', 'ô', 'ö', 'ù', 'û', 'ü', 'ÿ', 'œ', 'æ',
    ];

    /// Détection heuristique hors moteur TTS : accents FR et stopwords des deux langues.
    pub fn detect_language(text: &str) -> LanguageDetection {
        let lower = text.to_lowercase();
        let mut fr_score = 0.0f64;
        let mut en_score = 0.0f64;

        // 1) Accents français (poids forts)
        fr_score += lower.chars().filter(|c| FR_ACCENTS.contains(c)).count() as f64 * 3.0;

        // 2) Stopwords (poids 2 par occurrence)
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|w| !w.is_empty())
            .collect();
        let token_count = words.len();

        for word in &words {
            if FR_STOPWORDS.contains(word) {
                fr_score += 2.0;
            }
            if EN_STOPWORDS.contains(word) {
                en_score += 2.0;
            }
        }

        // 3) Français : apostrophe fréquente (l', d', n', s'…) — accent indirect
        let apostrophes = lower.matches('\'').count();
        fr_score += apostrophes as f64 * 1.5;

        if token_count < 3 || (fr_score == 0.0 && en_score == 0.0) {
            return LanguageDetection::Unknown;
        }

        let max = fr_score.max(en_score);
        let min = fr_score.min(en_score);
        // Mixte si aucun gagnant clair (le perdant pèse ≥ 35 % du gagnant)
        if min > 0.0 && min / max >= 0.35 {
            return LanguageDetection::Mixed;
        }
        if fr_score > en_score {
            LanguageDetection::French
        } else {
            LanguageDetection::English
        }
    }
}

pub mod segmentation;
pub use language::{detect_language, LanguageDetection};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ws_espaces_et_controles() {
        assert_eq!(normalize_ws("  Hello\u{00A0}World\x00  "), "Hello World");
        assert_eq!(normalize_ws("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(normalize_ws("  texte  \t ici  "), "texte ici");
    }

    #[test]
    fn html_fragment_reel() {
        let html = "<html><head><title>ignore</title></head><body><!--StartFragment--><p>Bonjour &amp; bienvenue</p><!--EndFragment--></body></html>";
        assert_eq!(clean_captured_text(html, "html").unwrap(), "Bonjour & bienvenue");
    }

    #[test]
    fn html_script_et_style_supprimes() {
        let html = "<p>visible</p><script>var secret = 1;</script><style>.x { color: red }</style><p>fin</p>";
        let cleaned = html_to_text(html);
        assert!(cleaned.contains("visible"));
        assert!(cleaned.contains("fin"));
        assert!(!cleaned.contains("secret"));
        assert!(!cleaned.contains("color"));
    }

    #[test]
    fn html_img_base64_supprimee() {
        let huge: String = "A".repeat(500);
        let html = format!("<div>texte avant<img src=\"data:image/png;base64,{huge}\"> texte après</div>");
        let cleaned = html_to_text(&html);
        assert!(!cleaned.contains("base64"));
        assert!(!cleaned.contains(&huge));
        assert!(cleaned.contains("texte avant"));
        assert!(cleaned.contains("texte après"));
    }

    #[test]
    fn html_balises_de_bloc_donnent_des_sauts_de_ligne() {
        let html = "<p>un</p><p>deux</p><div>trois<br>quatre</div>";
        let out = html_to_text(html);
        assert!(out.contains("un"));
        assert!(out.contains("deux"));
        assert!(out.contains("trois"));
        assert!(out.contains("quatre"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn html_lien_texte_conserve() {
        let html = "<a href=\"https://example.com/secret\">Documentation</a>";
        let cleaned = html_to_text(html);
        assert!(cleaned.contains("Documentation"));
    }

    #[test]
    fn html_data_uri_supprimee() {
        let html = "<img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==\">";
        assert!(!clean_captured_text(html, "html").is_ok());
    }

    #[test]
    fn html_vide_donne_erreur_claire() {
        let err = clean_captured_text("<div>   </div>", "html").unwrap_err();
        assert_eq!(err, CleanError::NoText);
        assert!(err.to_string().contains("Aucun texte exploitable"));
    }

    #[test]
    fn html_graphemes_html_decodes() {
        assert_eq!(html_to_text("&lt;cmd&gt; &amp; &quot;quoted&quot; &#39;sq&#39;"), "<cmd> & \"quoted\" 'sq'");
    }

    #[test]
    fn rtf_simple() {
        let rtf = r"{\rtf1\ansi{\fonttbl{\f0 Arial}} Texte de test\par fin}";
        let out = rtf_to_text(rtf);
        assert!(out.contains("Texte de test"));
        assert!(out.contains("fin"));
        assert!(!out.contains("fonttbl"));
        assert!(!out.contains("Arial"));
    }

    #[test]
    fn urls_nues_supprimees() {
        assert_eq!(strip_urls("voir https://example.com pour plus infos"), "voir pour plus infos");
        assert_eq!(strip_urls("fichier www.example.fr ok"), "fichier ok");
    }

    #[test]
    fn abreviations_fr() {
        assert_eq!(expand_abbrev_fr("M. Dupont"), "Monsieur Dupont");
        assert_eq!(expand_abbrev_fr("Mme Martin"), "Madame Martin");
        assert_eq!(expand_abbrev_fr("et M. Dupont a dit"), "et Monsieur Dupont a dit");
        // pas de faux positif dans un mot composé
        assert_eq!(expand_abbrev_fr("num.ro"), "num.ro");
    }

    #[test]
    fn language_francais_pur() {
        assert_eq!(detect_language("Bonjour le monde, c'est un test très bien écrit"), LanguageDetection::French);
        assert_eq!(detect_language("Il fait beau aujourd'hui et nous sommes très contents de vous"), LanguageDetection::French);
    }

    #[test]
    fn language_anglais_pur() {
        assert_eq!(detect_language("The quick brown fox jumps over the lazy dog"), LanguageDetection::English);
        assert_eq!(detect_language("This is a simple test of the language detection system"), LanguageDetection::English);
    }

    #[test]
    fn language_texte_mixte() {
        let det = detect_language("C'est un excellent commit, the code is clean and it works");
        assert_eq!(det, LanguageDetection::Mixed);
    }

    #[test]
    fn language_texte_trop_court_ou_sans_indice() {
        assert_eq!(detect_language("12345 999"), LanguageDetection::Unknown);
        assert_eq!(detect_language(""), LanguageDetection::Unknown);
        assert_eq!(detect_language("ab"), LanguageDetection::Unknown);
    }

    #[test]
    fn language_tts_hint_et_label() {
        assert_eq!(LanguageDetection::French.as_tts_hint(), "fr");
        assert_eq!(LanguageDetection::English.as_tts_hint(), "en");
        assert_eq!(LanguageDetection::Mixed.as_tts_hint(), "fr");
        assert_eq!(LanguageDetection::Unknown.as_tts_hint(), "fr");
        assert_eq!(LanguageDetection::French.label(), "fr");
        assert_eq!(LanguageDetection::Mixed.label(), "mixed");
    }

    #[test]
    fn language_francais_accents_fortes() {
        assert_eq!(detect_language("Voilà, c'est à côté de l'hôpital où j'habite régulièrement"), LanguageDetection::French);
    }

    #[test]
    fn clean_text_plain() {
        assert_eq!(clean_captured_text("  Hello\n\n\nWorld  ", "unicode").unwrap(), "Hello\n\nWorld");
    }
}
