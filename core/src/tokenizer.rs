/// Tokenize text into searchable tokens.
///
/// Conversion rules:
/// - Lowercases all characters
/// - Removes punctuation
/// - Splits on whitespace
/// - Filters out empty tokens
///
/// # Examples
///
/// ```
/// use pelisearch_core::tokenizer::tokenize;
///
/// let tokens = tokenize("Electric Bike For Commuting!");
/// assert_eq!(tokens, vec!["electric", "bike", "for", "commuting"]);
/// ```
pub fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_punctuation() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_text() {
        let tokens = tokenize("Hello World");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn removes_punctuation() {
        let tokens = tokenize("hello, world! how's it?");
        assert_eq!(tokens, vec!["hello", "world", "how", "s", "it"]);
    }

    #[test]
    fn empty_string_returns_empty_vec() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn only_punctuation_returns_empty_vec() {
        let tokens = tokenize("!!!,;.");
        assert!(tokens.is_empty());
    }

    #[test]
    fn unicode_text_does_not_panic() {
        let tokens = tokenize("café français 中文 español");
        // Should not panic — unicode is preserved
        assert!(tokens.contains(&"café".to_string()));
        assert!(tokens.contains(&"français".to_string()));
        assert!(tokens.contains(&"中文".to_string()));
        assert!(tokens.contains(&"español".to_string()));
    }

    #[test]
    fn multiple_whitespace_handled() {
        let tokens = tokenize("hello    world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn leading_trailing_whitespace_handled() {
        let tokens = tokenize("   hello world   ");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn mixed_punctuation_and_text() {
        let tokens = tokenize("Electric Bike For Commuting!");
        assert_eq!(tokens, vec!["electric", "bike", "for", "commuting"]);
    }

    #[test]
    fn numbers_preserved() {
        let tokens = tokenize("item 42");
        assert_eq!(tokens, vec!["item", "42"]);
    }
}
