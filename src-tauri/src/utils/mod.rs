//! Utilitaires communs

use std::path::PathBuf;

/// Retourne le chemin du répertoire de données de l'application
pub fn app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("Navi"))
}

/// Retourne le chemin du répertoire des modèles
pub fn models_dir() -> Option<PathBuf> {
    app_data_dir().map(|d| d.join("models"))
}

/// Nettoie et normalise un texte capturé (délègue à `text::normalize_ws`)
pub fn clean_text(text: &str) -> String {
    crate::text::normalize_ws(text)
}

/// Détecte la langue probable d'un texte (heuristique simple)
pub fn detect_language(text: &str) -> &'static str {
    let french_words = ["le", "la", "les", "un", "une", "des", "et", "ou", "mais", "donc", "car", "dans", "sur", "pour", "avec", "sans", "est", "sont", "était", "avez", "avez"];
    let english_words = ["the", "and", "or", "but", "so", "for", "in", "on", "at", "to", "of", "a", "an", "is", "are", "was", "were", "have", "has"];
    
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower.split_whitespace().collect();
    
    let fr_count = words.iter().filter(|w| french_words.contains(w)).count();
    let en_count = words.iter().filter(|w| english_words.contains(w)).count();
    
    if fr_count > en_count { "fr" } else { "en" }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("  Hello\x00World\n  "), "HelloWorld");
        assert_eq!(clean_text("\t\tTest\n\n"), "Test");
    }
    
    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("Hello world this is a test"), "en");
        assert_eq!(detect_language("Bonjour le monde c'est un test"), "fr");
    }
}