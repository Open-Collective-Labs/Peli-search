use std::collections::HashMap;

use crate::ranking::scorer::Scorer;
use crate::search;
use crate::storage::segment::IndexSegmentData;
use crate::storage::segment_searcher::{build_merged_stats, SegmentSearchResult};
use crate::types::SearchResult;

/// Merges search results from multiple segments into a single ranked list.
///
/// When a query spans multiple segments, each segment produces its own
/// BM25-scored results using local statistics. The `ResultMerger` combines
/// these into a globally consistent ranking by:
///
/// 1. Collecting all unique documents across segments
/// 2. Building merged collection statistics (global IDF, avg doc length)
/// 3. Re-scoring every candidate document with the global statistics
/// 4. Sorting by the re-computed scores
///
/// # Why re-scoring is necessary
///
/// BM25 scoring depends on collection-level statistics (total documents,
/// document frequency per term, average document length). A document that
/// appears in a small segment gets a different score than the same document
/// scored against global statistics. Re-scoring ensures fair ranking.
///
/// # Search Flow
///
/// ```text
/// Query
///   ├── Segment 1 → per-segment results
///   ├── Segment 2 → per-segment results
///   └── Segment 3 → per-segment results
///          ↓
///   ResultMerger::merge()
///          ↓
///   Collect unique doc IDs
///          ↓
///   Build global stats from all segment data
///          ↓
///   Re-score with global BM25
///          ↓
///   Sort by final score
///          ↓
///   Final ranked results
/// ```
pub struct ResultMerger;

impl ResultMerger {
    /// Merge per-segment search results into a single ranked list.
    ///
    /// Uses the provided segment data to build global statistics, then
    /// re-scores all candidate documents for accurate cross-segment ranking.
    ///
    /// # Arguments
    ///
    /// * `segment_results` - Per-segment search results from `SegmentSearcher`
    /// * `segment_data` - The full data for each segment (for building global stats)
    /// * `query` - The original query string (for re-scoring)
    ///
    /// # Returns
    ///
    /// A merged, globally-ranked list of `SearchResult` entries, sorted by
    /// score descending with stable doc_id tiebreaking.
    pub fn merge(
        segment_results: &[SegmentSearchResult],
        segment_data: &[IndexSegmentData],
        query: &str,
    ) -> Vec<SearchResult> {
        if segment_results.is_empty() {
            return Vec::new();
        }

        if query.is_empty() {
            return Vec::new();
        }

        // Step 1: Collect all unique document IDs across segments
        let candidates = collect_unique_candidates(segment_results);
        if candidates.is_empty() {
            return Vec::new();
        }

        // Step 2: Build global merged statistics from all segment data
        let global_stats = build_merged_stats(segment_data);

        // Step 3: Build a global inverted index for re-scoring
        let global_index = build_global_inverted_index(segment_data);

        // Step 4: Re-score all candidates using global statistics
        let scorer = Scorer::new(&global_stats);
        let candidate_refs: Vec<&str> = candidates.iter().map(|s| s.as_str()).collect();
        let mut results = scorer.score_documents(query, &candidate_refs);

        // Also score via the global inverted index to pick up documents
        // that may not be in the candidates list from segment results
        // (e.g., documents added during a race).
        let query_results = search::search(&global_index, &global_stats, query);
        for qr in query_results {
            if !results.iter().any(|r| r.document_id == qr.document_id) {
                results.push(qr);
            }
        }

        // Step 5: Sort by score descending (stable doc_id tiebreaker)
        sort_results(&mut results);

        results
    }

    /// Merge without segment data — uses only the per-segment results.
    ///
    /// This is a simpler merge that deduplicates by document ID and takes
    /// the highest score seen for each document. Less accurate than `merge()`
    /// because it doesn't re-score with global statistics, but useful when
    /// segment data is not available.
    pub fn merge_simple(segment_results: &[SegmentSearchResult]) -> Vec<SearchResult> {
        if segment_results.is_empty() {
            return Vec::new();
        }

        let mut best_scores: HashMap<String, f64> = HashMap::new();

        for seg_result in segment_results {
            for result in &seg_result.results {
                let entry = best_scores
                    .entry(result.document_id.clone())
                    .or_insert(0.0);
                if result.score > *entry {
                    *entry = result.score;
                }
            }
        }

        let mut results: Vec<SearchResult> = best_scores
            .into_iter()
            .map(|(id, score)| SearchResult::new(id, score))
            .collect();

        sort_results(&mut results);
        results
    }
}

/// Collect unique document IDs from per-segment results, preserving
/// insertion order (first occurrence wins).
fn collect_unique_candidates(segment_results: &[SegmentSearchResult]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();

    for seg_result in segment_results {
        for result in &seg_result.results {
            if seen.insert(result.document_id.clone()) {
                candidates.push(result.document_id.clone());
            }
        }
    }

    candidates
}

/// Build a global inverted index from all segment data.
///
/// Merges postings lists from all segments. When the same term appears
/// in multiple segments, the postings lists are concatenated (with
/// deduplication).
fn build_global_inverted_index(
    segments: &[IndexSegmentData],
) -> crate::index::InvertedIndex {
    let mut global = crate::index::InvertedIndex::new();

    for data in segments {
        // Re-index all documents from each segment into the global index
        for (doc_id, doc) in &data.documents {
            let text = extract_document_text(doc);
            let _ = global.add_document(doc_id, &text);
        }
    }

    global
}

/// Extract searchable text from a document's fields.
fn extract_document_text(doc: &crate::document::Document) -> String {
    let mut parts: Vec<String> = Vec::new();
    for value in doc.fields.values() {
        match value {
            serde_json::Value::String(s) => parts.push(s.clone()),
            serde_json::Value::Number(n) => parts.push(n.to_string()),
            serde_json::Value::Bool(b) => parts.push(b.to_string()),
            serde_json::Value::Array(arr) => {
                for v in arr {
                    if let serde_json::Value::String(s) = v {
                        parts.push(s.clone());
                    }
                }
            }
            _ => {}
        }
    }
    parts.join(" ")
}

/// Sort results by score descending with stable doc_id tiebreaker.
fn sort_results(results: &mut [SearchResult]) {
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.document_id.cmp(&b.document_id))
    });
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::ranking::statistics::CollectionStats;
    use crate::schema::Mapping;
    use crate::storage::segment::IndexSegmentData;
    use crate::storage::segment_searcher::SegmentSearchResult;

    use super::*;

    fn make_doc(id: &str, title: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), serde_json::json!(title));
        Document::new(id, fields).unwrap()
    }

    fn make_segment_data(name: &str, docs: Vec<(&str, &str)>) -> IndexSegmentData {
        let mut documents = HashMap::new();
        let mut index = crate::index::Index::new(name, Mapping::new(vec![]));

        for (id, title) in &docs {
            let doc = make_doc(id, title);
            documents.insert(id.to_string(), doc.clone());
            let _ = index.add_document(doc);
        }

        IndexSegmentData {
            name: name.to_string(),
            mapping: Mapping::new(vec![]),
            documents,
            inverted_index: index.inverted_index_clone(),
            stats: index.stats_ref().clone(),
        }
    }

    fn search_segment(data: &IndexSegmentData, seg_id: u64, query: &str) -> SegmentSearchResult {
        let results = crate::search::search(&data.inverted_index, &data.stats, query);
        SegmentSearchResult {
            segment_id: seg_id,
            results,
            stats: data.stats.clone(),
        }
    }

    #[test]
    fn merge_empty_results() {
        let merged = ResultMerger::merge(&[], &[], "hello");
        assert!(merged.is_empty());
    }

    #[test]
    fn merge_empty_query() {
        let data = make_segment_data("test", vec![("doc1", "hello world")]);
        let seg_results = vec![search_segment(&data, 1, "hello")];
        let merged = ResultMerger::merge(&seg_results, &[data], "");
        assert!(merged.is_empty());
    }

    #[test]
    fn merge_single_segment() {
        let data = make_segment_data("test", vec![("doc1", "hello world"), ("doc2", "hello there")]);
        let seg_results = vec![search_segment(&data, 1, "hello")];
        let merged = ResultMerger::merge(&seg_results, &[data], "hello");
        assert_eq!(merged.len(), 2);
        // Both match "hello", sorted by score desc
        assert!(merged[0].score >= merged[1].score);
    }

    #[test]
    fn merge_deduplicates_across_segments() {
        let data1 = make_segment_data("test", vec![("doc1", "hello world")]);
        let data2 = make_segment_data("test", vec![("doc1", "hello again")]);

        let seg1 = search_segment(&data1, 1, "hello");
        let seg2 = search_segment(&data2, 2, "hello");

        let merged = ResultMerger::merge(&[seg1, seg2], &[data1, data2], "hello");
        // doc1 appears in both segments but should appear only once
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].document_id, "doc1");
    }

    #[test]
    fn merge_different_documents_in_different_segments() {
        let data1 = make_segment_data("test", vec![("doc1", "hello world")]);
        let data2 = make_segment_data("test", vec![("doc2", "hello there")]);

        let seg1 = search_segment(&data1, 1, "hello");
        let seg2 = search_segment(&data2, 2, "hello");

        let merged = ResultMerger::merge(&[seg1, seg2], &[data1, data2], "hello");
        assert_eq!(merged.len(), 2);
        let ids: Vec<&str> = merged.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc1"));
        assert!(ids.contains(&"doc2"));
    }

    #[test]
    fn merge_preserves_ranking_order() {
        // Create segments with documents of different lengths
        // Shorter docs should score higher for the same query term
        let data1 = make_segment_data("test", vec![("doc1", "hello world")]);
        let data2 = make_segment_data("test", vec![("doc2", "hello")]);

        let seg1 = search_segment(&data1, 1, "hello");
        let seg2 = search_segment(&data2, 2, "hello");

        let merged = ResultMerger::merge(&[seg1, seg2], &[data1, data2], "hello");
        assert_eq!(merged.len(), 2);
        // doc2 is shorter, should score higher
        assert!(
            merged[0].score >= merged[1].score,
            "results should be sorted by score descending"
        );
    }

    #[test]
    fn merge_recomputes_scores_with_global_stats() {
        // When doc1 is in a tiny segment, its local score may differ from
        // the global score. The merger should re-score with global stats.
        let data1 = make_segment_data("test", vec![("doc1", "hello")]);
        let data2 = make_segment_data("test", vec![("doc2", "hello"), ("doc3", "hello")]);

        let seg1 = search_segment(&data1, 1, "hello");
        let seg2 = search_segment(&data2, 2, "hello");

        let merged = ResultMerger::merge(&[seg1, seg2], &[data1, data2], "hello");
        assert_eq!(merged.len(), 3);
        // All docs have the same content, so scores should be equal
        // (same doc length, same global stats)
        assert!((merged[0].score - merged[1].score).abs() < 1e-10);
        assert!((merged[1].score - merged[2].score).abs() < 1e-10);
    }

    #[test]
    fn merge_simple_deduplicates() {
        let seg_results = vec![
            SegmentSearchResult {
                segment_id: 1,
                results: vec![
                    SearchResult::new("doc1", 1.0),
                    SearchResult::new("doc2", 0.5),
                ],
                stats: CollectionStats::new(),
            },
            SegmentSearchResult {
                segment_id: 2,
                results: vec![
                    SearchResult::new("doc1", 0.8), // duplicate, lower score
                    SearchResult::new("doc3", 0.3),
                ],
                stats: CollectionStats::new(),
            },
        ];

        let merged = ResultMerger::merge_simple(&seg_results);
        assert_eq!(merged.len(), 3);

        // doc1 should have the highest score (1.0, not 0.8)
        let doc1 = merged.iter().find(|r| r.document_id == "doc1").unwrap();
        assert!((doc1.score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn merge_simple_sorts_by_score() {
        let seg_results = vec![
            SegmentSearchResult {
                segment_id: 1,
                results: vec![
                    SearchResult::new("low", 0.1),
                    SearchResult::new("high", 2.0),
                    SearchResult::new("mid", 1.0),
                ],
                stats: CollectionStats::new(),
            },
        ];

        let merged = ResultMerger::merge_simple(&seg_results);
        assert_eq!(merged[0].document_id, "high");
        assert_eq!(merged[1].document_id, "mid");
        assert_eq!(merged[2].document_id, "low");
    }

    #[test]
    fn merge_simple_empty() {
        let merged = ResultMerger::merge_simple(&[]);
        assert!(merged.is_empty());
    }

    #[test]
    fn merge_no_results_from_any_segment() {
        let data = make_segment_data("test", vec![("doc1", "hello world")]);
        let seg_results = vec![search_segment(&data, 1, "nonexistent")];
        let merged = ResultMerger::merge(&seg_results, &[data], "nonexistent");
        assert!(merged.is_empty());
    }

    #[test]
    fn merge_three_segments() {
        let data1 = make_segment_data("test", vec![("doc1", "electric bike")]);
        let data2 = make_segment_data("test", vec![("doc2", "electric car")]);
        let data3 = make_segment_data("test", vec![("doc3", "bike rack")]);

        let seg1 = search_segment(&data1, 1, "electric");
        let seg2 = search_segment(&data2, 2, "electric");
        let seg3 = search_segment(&data3, 3, "electric");

        let merged = ResultMerger::merge(&[seg1, seg2, seg3], &[data1, data2, data3], "electric");
        // doc1 and doc2 match "electric", doc3 does not
        assert_eq!(merged.len(), 2);
        let ids: Vec<&str> = merged.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc1"));
        assert!(ids.contains(&"doc2"));
    }

    #[test]
    fn merge_multi_term_query() {
        let data1 = make_segment_data("test", vec![("doc1", "electric bike review")]);
        let data2 = make_segment_data("test", vec![("doc2", "electric car review")]);

        let seg1 = search_segment(&data1, 1, "electric bike");
        let seg2 = search_segment(&data2, 2, "electric bike");

        let merged = ResultMerger::merge(&[seg1, seg2], &[data1, data2], "electric bike");
        // Only doc1 contains both "electric" AND "bike"
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].document_id, "doc1");
    }

    #[test]
    fn collect_unique_candidates_preserves_order() {
        let seg_results = vec![
            SegmentSearchResult {
                segment_id: 1,
                results: vec![
                    SearchResult::new("doc_c", 1.0),
                    SearchResult::new("doc_a", 0.5),
                ],
                stats: CollectionStats::new(),
            },
            SegmentSearchResult {
                segment_id: 2,
                results: vec![
                    SearchResult::new("doc_b", 0.8),
                    SearchResult::new("doc_a", 0.3), // duplicate
                ],
                stats: CollectionStats::new(),
            },
        ];

        let candidates = collect_unique_candidates(&seg_results);
        assert_eq!(candidates, vec!["doc_c", "doc_a", "doc_b"]);
    }
}
