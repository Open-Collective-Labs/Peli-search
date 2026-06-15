use crate::index::InvertedIndex;
use crate::ranking::explanation::{explain_document, ScoreExplanation};
use crate::ranking::scorer::Scorer;
use crate::ranking::statistics::CollectionStats;
use crate::tokenizer::tokenize;
use crate::types::{SearchResponse, SearchResult};

/// Search an inverted index for documents matching the query using BM25 ranking.
///
/// The query is tokenized and each token is looked up in the index.
/// Results are scored using BM25 and returned in descending order of score.
///
/// # Examples
///
/// ```
/// use pelisearch_core::index::InvertedIndex;
/// use pelisearch_core::ranking::statistics::CollectionStats;
/// use pelisearch_core::search::search;
///
/// let mut index = InvertedIndex::new();
/// let mut stats = CollectionStats::new();
///
/// index.add_document("doc_a", "electric bike").unwrap();
/// stats.update_document("doc_a", "electric bike");
/// index.add_document("doc_b", "electric").unwrap();
/// stats.update_document("doc_b", "electric");
///
/// let results = search(&index, &stats, "electric bike");
/// assert_eq!(results.len(), 2);
/// assert!(results[0].score > results[1].score);
/// ```
pub fn search(
    index: &InvertedIndex,
    stats: &CollectionStats,
    query: &str,
) -> Vec<SearchResult> {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }

    // Collect unique candidate doc IDs from all matching terms
    let mut candidate_set: Vec<String> = Vec::new();
    for token in &query_tokens {
        if let Some(postings) = index.get_postings(token) {
            for doc_id in postings {
                if !candidate_set.contains(doc_id) {
                    candidate_set.push(doc_id.clone());
                }
            }
        }
    }

    if candidate_set.is_empty() {
        return Vec::new();
    }

    let doc_ids: Vec<&str> = candidate_set.iter().map(|s| s.as_str()).collect();
    let scorer = Scorer::new(stats);
    scorer.score_documents(query, &doc_ids)
}

/// Search with BM25 ranking and return per-document score explanations.
///
/// # Examples
///
/// ```
/// use pelisearch_core::index::InvertedIndex;
/// use pelisearch_core::ranking::statistics::CollectionStats;
/// use pelisearch_core::search::search_with_explanations;
///
/// let mut index = InvertedIndex::new();
/// let mut stats = CollectionStats::new();
///
/// index.add_document("doc_a", "electric bike").unwrap();
/// stats.update_document("doc_a", "electric bike");
///
/// let response = search_with_explanations(&index, &stats, "electric");
/// assert_eq!(response.results.len(), 1);
/// assert!(!response.explanations.is_empty());
/// ```
pub fn search_with_explanations(
    index: &InvertedIndex,
    stats: &CollectionStats,
    query: &str,
) -> SearchResponse {
    let results = search(index, stats, query);

    let explanations: Vec<(String, Vec<ScoreExplanation>)> = results
        .iter()
        .map(|r| {
            let exps = explain_document(query, &r.document_id, stats);
            (r.document_id.clone(), exps)
        })
        .collect();

    SearchResponse::new(results, explanations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::InvertedIndex;

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

    fn setup_stats() -> CollectionStats {
        let mut stats = CollectionStats::new();
        stats.update_document("doc_a", "electric bike");
        stats.update_document("doc_b", "electric");
        stats.update_document("doc_c", "bike for commuting");
        stats.update_document("doc_d", "walking");
        stats
    }

    #[test]
    fn matching_documents_returned() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "electric");
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc_a"));
        assert!(ids.contains(&"doc_b"));
    }

    #[test]
    fn results_sorted_by_score_descending() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "electric bike");

        assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn higher_score_for_more_terms() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "electric bike");

        let doc_a = results.iter().find(|r| r.document_id == "doc_a").unwrap();
        let doc_b = results.iter().find(|r| r.document_id == "doc_b").unwrap();
        let doc_c = results.iter().find(|r| r.document_id == "doc_c").unwrap();

        // doc_a matches both terms, doc_b and doc_c match one
        assert!(doc_a.score > doc_b.score);
        assert!(doc_a.score > doc_c.score);
    }

    #[test]
    fn empty_query_returns_empty_results() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "");
        assert!(results.is_empty());
    }

    #[test]
    fn nonexistent_terms_return_empty() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "zzzzzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn query_with_only_punctuation_returns_empty() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "!!! ???");
        assert!(results.is_empty());
    }

    #[test]
    fn case_insensitive_search() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "ELECTRIC BIKE");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn partial_match_works() {
        let mut index = InvertedIndex::new();
        let mut stats = CollectionStats::new();
        index.add_document("doc1", "the quick brown fox").unwrap();
        stats.update_document("doc1", "the quick brown fox");
        index.add_document("doc2", "jumps over the lazy dog").unwrap();
        stats.update_document("doc2", "jumps over the lazy dog");

        let results = search(&index, &stats, "quick fox");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn multiple_documents_same_score() {
        let mut index = InvertedIndex::new();
        let mut stats = CollectionStats::new();
        index.add_document("doc_a", "hello").unwrap();
        stats.update_document("doc_a", "hello");
        index.add_document("doc_b", "hello").unwrap();
        stats.update_document("doc_b", "hello");
        index.add_document("doc_c", "hello").unwrap();
        stats.update_document("doc_c", "hello");

        let results = search(&index, &stats, "hello");
        assert_eq!(results.len(), 3);
        // All docs contain "hello" once, same length — scores should be equal
        assert_eq!(results[0].score, results[1].score);
        assert_eq!(results[1].score, results[2].score);
    }

    #[test]
    fn search_with_explanations_returns_explanations() {
        let index = setup_index();
        let stats = setup_stats();
        let response = search_with_explanations(&index, &stats, "electric bike");

        assert_eq!(response.results.len(), response.explanations.len());
        for (doc_id, exps) in &response.explanations {
            // Each result should have at least one non-zero explanation
            assert!(!exps.is_empty());
            for exp in exps {
                assert_eq!(exp.term, exp.term.to_lowercase());
                assert!(exp.contribution >= 0.0);
            }
            // Verify doc_id in results
            assert!(response.results.iter().any(|r| &r.document_id == doc_id));
        }
    }

    #[test]
    fn nonexistent_terms_no_explanations() {
        let index = setup_index();
        let stats = setup_stats();
        let response = search_with_explanations(&index, &stats, "zzzzz");
        assert!(response.results.is_empty());
        assert!(response.explanations.is_empty());
    }
}
