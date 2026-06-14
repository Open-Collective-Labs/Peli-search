use std::collections::HashMap;

use crate::index::InvertedIndex;
use crate::tokenizer::tokenize;
use crate::types::SearchResult;

/// Search an inverted index for documents matching the query.
///
/// The query is tokenized and each token is looked up in the index.
/// Results are scored by the number of matching query terms and
/// returned in descending order of score.
///
/// # Examples
///
/// ```
/// use pelisearch_core::index::InvertedIndex;
/// use pelisearch_core::search::search;
///
/// let mut index = InvertedIndex::new();
/// index.add_document("doc_a", "electric bike").unwrap();
/// index.add_document("doc_b", "electric").unwrap();
///
/// let results = search(&index, "electric bike");
/// assert_eq!(results.len(), 2);
/// // doc_a matches both terms (score 2), doc_b matches one (score 1)
/// assert_eq!(results[0].document_id, "doc_a");
/// assert_eq!(results[0].score, 2.0);
/// assert_eq!(results[1].document_id, "doc_b");
/// assert_eq!(results[1].score, 1.0);
/// ```
pub fn search(index: &InvertedIndex, query: &str) -> Vec<SearchResult> {
    let tokens = tokenize(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut scores: HashMap<String, usize> = HashMap::new();

    for token in &tokens {
        if let Some(postings) = index.get_postings(token) {
            for doc_id in postings {
                *scores.entry(doc_id.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut results: Vec<SearchResult> = scores
        .into_iter()
        .map(|(document_id, score)| SearchResult::new(document_id, score as f64))
        .collect();

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.document_id.cmp(&b.document_id))
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_index() -> InvertedIndex {
        let mut index = InvertedIndex::new();
        index.add_document("doc_a", "electric bike").unwrap();
        index.add_document("doc_b", "electric").unwrap();
        index
            .add_document("doc_c", "bike for commuting")
            .unwrap();
        index.add_document("doc_d", "walking").unwrap();
        index
    }

    #[test]
    fn matching_documents_returned() {
        let index = setup_index();
        let results = search(&index, "electric");
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc_a"));
        assert!(ids.contains(&"doc_b"));
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let index = setup_index();
        let results = search(&index, "electric bike");

        assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn higher_score_for_more_terms() {
        let index = setup_index();
        let results = search(&index, "electric bike");

        let doc_a = results.iter().find(|r| r.document_id == "doc_a").unwrap();
        let doc_b = results.iter().find(|r| r.document_id == "doc_b").unwrap();

        assert_eq!(doc_a.score, 2.0);
        assert_eq!(doc_b.score, 1.0);
    }

    #[test]
    fn empty_query_returns_empty_results() {
        let index = setup_index();
        let results = search(&index, "");
        assert!(results.is_empty());
    }

    #[test]
    fn nonexistent_terms_return_empty() {
        let index = setup_index();
        let results = search(&index, "zzzzzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn query_with_only_punctuation_returns_empty() {
        let index = setup_index();
        let results = search(&index, "!!! ???");
        assert!(results.is_empty());
    }

    #[test]
    fn case_insensitive_search() {
        let index = setup_index();
        let results = search(&index, "ELECTRIC BIKE");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].score, 2.0);
    }

    #[test]
    fn partial_match_works() {
        let mut index = InvertedIndex::new();
        index.add_document("doc1", "the quick brown fox").unwrap();
        index.add_document("doc2", "jumps over the lazy dog").unwrap();

        let results = search(&index, "quick fox");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
        assert_eq!(results[0].score, 2.0);
    }

    #[test]
    fn multiple_documents_same_score() {
        let mut index = InvertedIndex::new();
        index.add_document("doc_a", "hello").unwrap();
        index.add_document("doc_b", "hello").unwrap();
        index.add_document("doc_c", "hello").unwrap();

        let results = search(&index, "hello");
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.score, 1.0);
        }
    }
}
