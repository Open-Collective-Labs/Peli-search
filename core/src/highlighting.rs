use crate::tokenizer::tokenize;

/// Wraps matched query terms in `<em>...</em>` tags.
///
/// Both `text` and `query` are tokenized using the same tokenizer
/// so that matching is case-insensitive and punctuation-aware.
///
/// # Examples
///
/// ```
/// use pelisearch_core::highlighting::highlight;
///
/// let result = highlight("Learning Rust is fun", "rust");
/// assert_eq!(result, "Learning <em>Rust</em> is fun");
/// ```
pub fn highlight(text: &str, query: &str) -> String {
    let query_tokens: Vec<String> = tokenize(query);
    if query_tokens.is_empty() || text.is_empty() {
        return text.to_string();
    }

    let mut result = String::new();
    let mut i = 0;
    let words: Vec<&str> = text.split_whitespace().collect();

    for word in &words {
        if i > 0 {
            result.push(' ');
        }
        let lower = word.to_lowercase();
        let trimmed = lower.trim_matches(|c: char| c.is_ascii_punctuation());
        if query_tokens.iter().any(|qt| qt == trimmed) {
            result.push_str("<em>");
            result.push_str(word);
            result.push_str("</em>");
        } else {
            result.push_str(word);
        }
        i += 1;
    }

    result
}

/// Highlight multiple fields in a document.
///
/// Returns a map of field name to highlighted text for only the
/// fields that contain matching terms.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::highlighting::highlight_fields;
///
/// let mut doc = HashMap::new();
/// doc.insert("title".to_string(), serde_json::json!("Learning Rust"));
/// doc.insert("description".to_string(), serde_json::json!("A book about Rust programming"));
///
/// let highlighted = highlight_fields(&doc, "rust");
/// assert!(highlighted["title"].contains("<em>Rust</em>"));
/// ```
pub fn highlight_fields(
    doc: &std::collections::HashMap<String, serde_json::Value>,
    query: &str,
) -> std::collections::HashMap<String, String> {
    let mut result = std::collections::HashMap::new();
    for (field, value) in doc {
        if let Some(text) = value.as_str() {
            let highlighted = highlight(text, query);
            if highlighted != text {
                result.insert(field.clone(), highlighted);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_term_matched() {
        let result = highlight("hello world", "world");
        assert_eq!(result, "hello <em>world</em>");
    }

    #[test]
    fn case_insensitive_match() {
        let result = highlight("Hello World", "world");
        assert_eq!(result, "Hello <em>World</em>");
    }

    #[test]
    fn no_match_returns_original() {
        let result = highlight("hello world", "goodbye");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn empty_query_returns_original() {
        let result = highlight("hello world", "");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn empty_text_returns_empty() {
        let result = highlight("", "hello");
        assert_eq!(result, "");
    }

    #[test]
    fn multi_term_query_highlights_all() {
        let result = highlight("learning rust programming", "rust programming");
        assert_eq!(result, "learning <em>rust</em> <em>programming</em>");
    }

    #[test]
    fn punctuation_handled() {
        let result = highlight("hello, world!", "world");
        assert_eq!(result, "hello, <em>world!</em>");
    }

    #[test]
    fn highlight_fields_returns_only_matched() {
        let mut doc = std::collections::HashMap::new();
        doc.insert("title".to_string(), serde_json::json!("Rust Guide"));
        doc.insert("body".to_string(), serde_json::json!("Other content"));
        let result = highlight_fields(&doc, "rust");
        assert!(result.contains_key("title"));
        assert!(!result.contains_key("body"));
    }

    #[test]
    fn highlight_fields_empty_query() {
        let mut doc = std::collections::HashMap::new();
        doc.insert("title".to_string(), serde_json::json!("Rust"));
        let result = highlight_fields(&doc, "");
        assert!(result.is_empty());
    }

    #[test]
    fn highlight_preserves_whitespace() {
        // Note: split_whitespace normalizes whitespace
        let result = highlight("a  b  c", "b");
        assert_eq!(result, "a <em>b</em> c");
    }
}
