use crate::ranking::bm25::BM25Config;
use crate::ranking::statistics::CollectionStats;
use crate::tokenizer::tokenize;

/// A detailed breakdown of how a single term contributed to a document's score.
///
/// # Examples
///
/// ```
/// use pelisearch_core::ranking::explanation::{ScoreExplanation, explain_document};
/// use pelisearch_core::ranking::statistics::CollectionStats;
///
/// let mut stats = CollectionStats::new();
/// stats.update_document("doc1", "bike bike review");
///
/// let explanations = explain_document("bike", "doc1", &stats);
/// assert_eq!(explanations.len(), 1);
/// assert_eq!(explanations[0].term, "bike");
/// assert_eq!(explanations[0].tf, 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreExplanation {
    /// The query term being explained.
    pub term: String,
    /// Number of times the term appears in the document.
    pub tf: u64,
    /// Inverse Document Frequency value.
    pub idf: f64,
    /// The term's contribution to the total score (idf * score_term).
    pub contribution: f64,
}

/// Generate per-term score explanations for a document against a query.
///
/// Returns a list of `ScoreExplanation` entries, one for each query term
/// that matched the document. Terms that do not appear in the document
/// are excluded.
///
/// # Examples
///
/// ```
/// use pelisearch_core::ranking::explanation::explain_document;
/// use pelisearch_core::ranking::statistics::CollectionStats;
///
/// let mut stats = CollectionStats::new();
/// stats.update_document("doc_a", "electric bike review");
/// stats.update_document("doc_b", "electric");
///
/// let explanations = explain_document("electric bike", "doc_a", &stats);
/// assert_eq!(explanations.len(), 2);
///
/// let bike_exp = explanations.iter().find(|e| e.term == "bike").unwrap();
/// assert!(bike_exp.contribution > 0.0);
/// ```
pub fn explain_document(query: &str, doc_id: &str, stats: &CollectionStats) -> Vec<ScoreExplanation> {
    let config = BM25Config::default();
    let total_docs = stats.total_documents();

    if total_docs == 0 || stats.get_doc_length(doc_id).is_none() {
        return Vec::new();
    }

    let avg_doc_len = stats.average_document_length() as f32;
    if avg_doc_len <= 0.0 {
        return Vec::new();
    }

    let doc_len = match stats.get_doc_length(doc_id) {
        Some(len) => len as f32,
        None => return Vec::new(),
    };

    let query_tokens = tokenize(query);
    let mut seen_terms = Vec::new();
    let mut explanations = Vec::new();

    for token in &query_tokens {
        if seen_terms.contains(token) {
            continue;
        }
        seen_terms.push(token.clone());

        let term_stats = match stats.get_term_stats(token) {
            Some(s) => s,
            None => continue,
        };

        let tf = stats.get_term_frequency_in_doc(token, doc_id);
        if tf == 0 {
            continue;
        }

        let idf = config.idf(term_stats.document_frequency, total_docs);
        let term_score = config.score_term(tf as f32, doc_len, avg_doc_len);
        let contribution = idf * term_score;

        explanations.push(ScoreExplanation {
            term: token.clone(),
            tf,
            idf: idf as f64,
            contribution: contribution as f64,
        });
    }

    explanations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_stats() -> CollectionStats {
        let mut stats = CollectionStats::new();
        stats.update_document("doc_a", "electric bike review");
        stats.update_document("doc_b", "electric");
        stats.update_document("doc_c", "bike for commuting");
        stats.update_document("doc_d", "walking in the park");
        stats
    }

    #[test]
    fn every_result_generates_explanation_data() {
        let stats = setup_stats();
        let explanations = explain_document("electric bike", "doc_a", &stats);

        assert_eq!(explanations.len(), 2);

        for exp in &explanations {
            assert!(!exp.term.is_empty());
            assert!(exp.tf > 0);
            assert!(exp.idf > 0.0);
            assert!(exp.contribution > 0.0);
        }
    }

    #[test]
    fn missing_terms_handled() {
        let stats = setup_stats();
        let explanations = explain_document("electric zzzz", "doc_a", &stats);

        // Only "electric" should appear, "zzzz" does not exist
        assert_eq!(explanations.len(), 1);
        assert_eq!(explanations[0].term, "electric");
    }

    #[test]
    fn missing_document_returns_empty() {
        let stats = setup_stats();
        let explanations = explain_document("electric", "nonexistent", &stats);
        assert!(explanations.is_empty());
    }

    #[test]
    fn empty_query_returns_empty() {
        let stats = setup_stats();
        let explanations = explain_document("", "doc_a", &stats);
        assert!(explanations.is_empty());
    }

    #[test]
    fn term_with_no_match_excluded() {
        let stats = setup_stats();
        let explanations = explain_document("walking", "doc_a", &stats);
        // doc_a does not contain "walking"
        assert!(explanations.is_empty());
    }

    #[test]
    fn tf_reflects_term_count_in_document() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "bike bike review");

        let explanations = explain_document("bike", "doc1", &stats);
        assert_eq!(explanations.len(), 1);
        assert_eq!(explanations[0].tf, 2);
    }

    #[test]
    fn multiple_terms_with_correct_contributions() {
        let stats = setup_stats();
        let explanations = explain_document("electric bike", "doc_a", &stats);

        assert_eq!(explanations.len(), 2);

        let total: f64 = explanations.iter().map(|e| e.contribution).sum();
        assert!(total > 0.0);
    }

    #[test]
    fn explanation_fields_are_populated() {
        let mut stats = CollectionStats::new();
        stats.update_document("doc1", "bike");

        let explanations = explain_document("bike", "doc1", &stats);
        assert_eq!(explanations.len(), 1);

        let exp = &explanations[0];
        assert_eq!(exp.term, "bike");
        assert_eq!(exp.tf, 1);
        assert!(exp.idf > 0.0);
        assert!(exp.contribution > 0.0);
    }
}
