use std::collections::HashSet;

use crate::index::InvertedIndex;
use crate::ranking::scorer::Scorer;
use crate::ranking::statistics::CollectionStats;
use crate::tokenizer::tokenize;
use crate::types::{AggregationResults, SearchHit, SearchResponse, SearchResult};

/// Search an inverted index for documents matching the query using BM25 ranking.
///
/// The query is tokenized and each token is looked up in the index.
/// For multi-term queries, only documents containing ALL terms are
/// considered as candidates (AND semantics). The smallest (rarest)
/// posting list is used as the base for intersection, minimizing
/// the number of documents that need to be scored.
///
/// Results are ranked by BM25 and returned in descending order of score.
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
/// // Only doc_a contains both "electric" AND "bike"
/// let results = search(&index, &stats, "electric bike");
/// assert_eq!(results.len(), 1);
/// assert_eq!(results[0].document_id, "doc_a");
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

    // Collect posting lists for all query tokens.
    // If any query token has no postings, no document can match all terms.
    let mut posting_lists: Vec<&Vec<String>> = Vec::with_capacity(query_tokens.len());
    for token in &query_tokens {
        match index.get_postings(token) {
            Some(list) => posting_lists.push(list),
            None => return Vec::new(), // term not found → no results
        }
    }

    // Find the smallest posting list (rarest term).
    // This minimizes the number of candidate documents we need to check.
    let smallest_idx = posting_lists
        .iter()
        .enumerate()
        .min_by_key(|(_, list)| list.len())
        .map(|(idx, _)| idx)
        .unwrap();

    let smallest_list = posting_lists[smallest_idx];

    // For single-term queries, the smallest list is the only list.
    // For multi-term queries, intersect with all other posting lists
    // (AND semantics — a doc must contain every term to be a candidate).
    // Use HashSet for O(1) containment checks instead of Vec::contains O(n).
    let candidates: Vec<&str> = if posting_lists.len() == 1 {
        smallest_list.iter().map(|s| s.as_str()).collect()
    } else {
        let other_sets: Vec<HashSet<&str>> = posting_lists
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != smallest_idx)
            .map(|(_, list)| list.iter().map(|s| s.as_str()).collect())
            .collect();

        smallest_list
            .iter()
            .filter(|doc_id| other_sets.iter().all(|set| set.contains(doc_id.as_str())))
            .map(|s| s.as_str())
            .collect()
    };

    if candidates.is_empty() {
        return Vec::new();
    }

    let scorer = Scorer::new(stats);
    scorer.score_documents(query, &candidates)
}

/// Search with OR semantics — returns documents containing ANY query term.
///
/// Unlike `search()` which uses AND semantics for multi-term queries,
/// this function returns all documents that match at least one term.
/// Useful for query pipelines that need broad candidate collection
/// before applying field filters or other constraints.
///
/// # Examples
///
/// ```
/// use pelisearch_core::index::InvertedIndex;
/// use pelisearch_core::ranking::statistics::CollectionStats;
/// use pelisearch_core::search::search_any;
///
/// let mut index = InvertedIndex::new();
/// let mut stats = CollectionStats::new();
///
/// index.add_document("doc_a", "electric bike").unwrap();
/// stats.update_document("doc_a", "electric bike");
/// index.add_document("doc_b", "electric").unwrap();
/// stats.update_document("doc_b", "electric");
/// index.add_document("doc_c", "bike").unwrap();
/// stats.update_document("doc_c", "bike");
///
/// // All docs that contain either "electric" OR "bike"
/// let results = search_any(&index, &stats, "electric bike");
/// assert_eq!(results.len(), 3);
/// ```
pub fn search_any(
    index: &InvertedIndex,
    stats: &CollectionStats,
    query: &str,
) -> Vec<SearchResult> {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }

    // Collect unique candidate doc IDs from all matching terms (OR semantics)
    // Use HashSet for O(1) dedup instead of Vec::contains O(n).
    let mut seen = HashSet::new();
    let mut candidates: Vec<String> = Vec::new();
    for token in &query_tokens {
        if let Some(postings) = index.get_postings(token) {
            for doc_id in postings {
                if seen.insert(doc_id.clone()) {
                    candidates.push(doc_id.clone());
                }
            }
        }
    }

    if candidates.is_empty() {
        return Vec::new();
    }

    let doc_ids: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
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
/// assert_eq!(response.hits.len(), 1);
/// assert!(response.aggregations.is_empty());
/// ```
pub fn search_with_explanations(
    index: &InvertedIndex,
    stats: &CollectionStats,
    query: &str,
) -> SearchResponse {
    let results = search(index, stats, query);
    let hits: Vec<SearchHit> = results
        .into_iter()
        .map(|r| SearchHit::new("", r.document_id, r.score))
        .collect();

    let total = hits.len();
    SearchResponse::new(hits, AggregationResults::new(), total)
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
    fn multi_term_and_semantics() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "electric bike");

        // With AND semantics, only doc_a contains both "electric" AND "bike"
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc_a");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn single_term_still_unions() {
        let index = setup_index();
        let stats = setup_stats();
        let results = search(&index, &stats, "electric");

        // Single term: all docs containing it are returned
        assert_eq!(results.len(), 2);
        let ids: Vec<&str> = results.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc_a"));
        assert!(ids.contains(&"doc_b"));
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
        // doc_a is the only doc with both "electric" AND "bike"
        let results = search(&index, &stats, "ELECTRIC BIKE");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc_a");
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
        // doc_a is the only doc with both "electric" AND "bike"
        let response = search_with_explanations(&index, &stats, "electric bike");

        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].document_id, "doc_a");
        assert!(response.aggregations.is_empty());
    }

    #[test]
    fn nonexistent_terms_no_explanations() {
        let index = setup_index();
        let stats = setup_stats();
        let response = search_with_explanations(&index, &stats, "zzzzz");
        assert!(response.hits.is_empty());
        assert!(response.aggregations.is_empty());
    }
}
