use std::io;
use std::path::Path;

use crate::ranking::statistics::CollectionStats;
use crate::search;
use crate::storage::result_merger::ResultMerger;
use crate::storage::segment::{IndexSegmentData, Segment};
use crate::types::SearchResult;

/// Per-segment search results before merging.
pub struct SegmentSearchResult {
    /// Segment ID this result came from.
    pub segment_id: u64,
    /// BM25-ranked results from this segment.
    pub results: Vec<SearchResult>,
    /// Statistics used to score this segment's results.
    pub stats: CollectionStats,
}

/// Searches across multiple immutable segments.
///
/// Each segment is searched independently using its own inverted index
/// and collection statistics. The results are intended to be merged by
/// [`ResultMerger`](super::result_merger::ResultMerger).
///
/// # Search Flow
///
/// 1. Load each segment's data from disk
/// 2. Run BM25 search on each segment independently
/// 3. Return per-segment results for downstream merging
pub struct SegmentSearcher;

impl SegmentSearcher {
    /// Search all segments in a directory, returning per-segment results.
    ///
    /// Each segment is searched independently. Empty or non-existent
    /// directories return an empty vec.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if any segment file cannot be read or
    /// deserialized.
    pub fn search_all(
        index_dir: &Path,
        query: &str,
    ) -> io::Result<Vec<SegmentSearchResult>> {
        let segments = Segment::discover(index_dir)?;
        Self::search_segments(&segments, query)
    }

    /// Search a specific set of segments (by their on-disk paths).
    ///
    /// Useful when the caller has already discovered segments and wants
    /// to search only a subset.
    pub fn search_segments(
        segments: &[Segment],
        query: &str,
    ) -> io::Result<Vec<SegmentSearchResult>> {
        let mut segment_results = Vec::new();

        for seg in segments {
            let data = Segment::read_segment(&seg.path)?;
            let results = Self::search_segment_data(&data, query);
            segment_results.push(SegmentSearchResult {
                segment_id: seg.id,
                results,
                stats: data.stats,
            });
        }

        Ok(segment_results)
    }

    /// Search a single segment's in-memory data.
    ///
    /// Returns BM25-ranked search results sorted by score descending.
    /// Returns an empty vec if the query is empty or no terms match.
    pub fn search_segment_data(
        data: &IndexSegmentData,
        query: &str,
    ) -> Vec<SearchResult> {
        search::search(&data.inverted_index, &data.stats, query)
    }

    /// Search across all segments and merge results with global re-ranking.
    ///
    /// This is the primary search entry point for multi-segment indexes.
    /// It searches each segment independently, then uses [`ResultMerger`]
    /// to produce a single globally-ranked result list.
    ///
    /// # Search Flow
    ///
    /// 1. Discover all segments in the directory
    /// 2. Search each segment independently (local BM25)
    /// 3. Load segment data for global statistics
    /// 4. Merge results with re-scoring using global BM25 statistics
    /// 5. Return globally-ranked results
    ///
    /// # Errors
    ///
    /// Returns an I/O error if any segment file cannot be read.
    pub fn search_merged(
        index_dir: &Path,
        query: &str,
    ) -> io::Result<Vec<SearchResult>> {
        let segments = Segment::discover(index_dir)?;
        if segments.is_empty() {
            return Ok(Vec::new());
        }

        // Load all segment data
        let mut segment_data: Vec<IndexSegmentData> = Vec::new();
        let mut segment_results: Vec<SegmentSearchResult> = Vec::new();

        for seg in &segments {
            let data = Segment::read_segment(&seg.path)?;
            let results = Self::search_segment_data(&data, query);
            segment_results.push(SegmentSearchResult {
                segment_id: seg.id,
                results,
                stats: data.stats.clone(),
            });
            segment_data.push(data);
        }

        // Merge with global re-ranking
        let merged = ResultMerger::merge(&segment_results, &segment_data, query);
        Ok(merged)
    }
}

/// Build merged collection statistics from all documents across segments.
///
/// This is necessary for proper BM25 re-ranking after merging, because
/// BM25's IDF and document-length normalization depend on global
/// collection statistics.
pub fn build_merged_stats(
    segments: &[IndexSegmentData],
) -> CollectionStats {
    let mut merged = CollectionStats::new();
    for data in segments {
        for (_id, doc) in &data.documents {
            let text = extract_document_text(doc);
            merged.update_document(&doc.id, &text);
        }
    }
    merged
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

/// Collect candidate document IDs from per-segment results in insertion
/// order (first occurrence wins for deduplication).
pub fn collect_candidates(
    segment_results: &[SegmentSearchResult],
) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::schema::Mapping;
    use crate::storage::segment::IndexSegmentData;

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

    #[test]
    fn search_single_segment() {
        let data = make_segment_data("test", vec![("doc1", "hello world")]);
        let results = SegmentSearcher::search_segment_data(&data, "hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let data = make_segment_data("test", vec![("doc1", "hello world")]);
        let results = SegmentSearcher::search_segment_data(&data, "");
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_match_returns_empty() {
        let data = make_segment_data("test", vec![("doc1", "hello world")]);
        let results = SegmentSearcher::search_segment_data(&data, "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn search_multiple_docs_in_segment() {
        let data = make_segment_data("test", vec![
            ("doc1", "hello world"),
            ("doc2", "hello there"),
            ("doc3", "goodbye"),
        ]);
        let results = SegmentSearcher::search_segment_data(&data, "hello");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_results_sorted_by_score() {
        let data = make_segment_data("test", vec![
            ("doc1", "hello world"),
            ("doc2", "hello"),
        ]);
        let results = SegmentSearcher::search_segment_data(&data, "hello");
        // Both match "hello", but doc2 is shorter so should score higher
        assert!(!results.is_empty());
        assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn search_across_segments() {
        let dir = tempfile::tempdir().unwrap();

        let seg1_data = make_segment_data("test", vec![("doc1", "hello world")]);
        Segment::write_segment(dir.path(), 1, &seg1_data).unwrap();

        let seg2_data = make_segment_data("test", vec![("doc2", "hello there")]);
        Segment::write_segment(dir.path(), 2, &seg2_data).unwrap();

        let results = SegmentSearcher::search_all(dir.path(), "hello").unwrap();
        assert_eq!(results.len(), 2);

        let seg1 = results.iter().find(|r| r.segment_id == 1).unwrap();
        assert_eq!(seg1.results.len(), 1);
        assert_eq!(seg1.results[0].document_id, "doc1");

        let seg2 = results.iter().find(|r| r.segment_id == 2).unwrap();
        assert_eq!(seg2.results.len(), 1);
        assert_eq!(seg2.results[0].document_id, "doc2");
    }

    #[test]
    fn search_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let results = SegmentSearcher::search_all(dir.path(), "hello").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn collect_candidates_deduplicates() {
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
                    SearchResult::new("doc1", 0.8), // duplicate
                    SearchResult::new("doc3", 0.3),
                ],
                stats: CollectionStats::new(),
            },
        ];

        let candidates = collect_candidates(&seg_results);
        assert_eq!(candidates, vec!["doc1", "doc2", "doc3"]);
    }

    #[test]
    fn build_merged_stats_from_segments() {
        let seg1 = make_segment_data("test", vec![("doc1", "hello world")]);
        let seg2 = make_segment_data("test", vec![("doc2", "hello there")]);

        let merged = build_merged_stats(&[seg1, seg2]);
        assert_eq!(merged.total_documents(), 2);
    }

    #[test]
    fn extract_text_from_document() {
        let doc = make_doc("doc1", "hello world");
        let text = extract_document_text(&doc);
        assert_eq!(text, "hello world");
    }

    #[test]
    fn search_merged_across_segments() {
        let dir = tempfile::tempdir().unwrap();

        let seg1_data = make_segment_data("test", vec![("doc1", "hello world")]);
        Segment::write_segment(dir.path(), 1, &seg1_data).unwrap();

        let seg2_data = make_segment_data("test", vec![("doc2", "hello there")]);
        Segment::write_segment(dir.path(), 2, &seg2_data).unwrap();

        let results = SegmentSearcher::search_merged(dir.path(), "hello").unwrap();
        assert_eq!(results.len(), 2);

        let ids: Vec<&str> = results.iter().map(|r| r.document_id.as_str()).collect();
        assert!(ids.contains(&"doc1"));
        assert!(ids.contains(&"doc2"));
    }

    #[test]
    fn search_merged_deduplicates_documents() {
        let dir = tempfile::tempdir().unwrap();

        let seg1_data = make_segment_data("test", vec![("doc1", "hello world")]);
        Segment::write_segment(dir.path(), 1, &seg1_data).unwrap();

        let seg2_data = make_segment_data("test", vec![("doc1", "hello again")]);
        Segment::write_segment(dir.path(), 2, &seg2_data).unwrap();

        let results = SegmentSearcher::search_merged(dir.path(), "hello").unwrap();
        assert_eq!(results.len(), 1, "doc1 should appear only once");
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn search_merged_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let results = SegmentSearcher::search_merged(dir.path(), "hello").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_merged_preserves_ranking() {
        let dir = tempfile::tempdir().unwrap();

        // Short doc should score higher than long doc
        let seg1_data = make_segment_data("test", vec![("doc1", "hello world")]);
        Segment::write_segment(dir.path(), 1, &seg1_data).unwrap();

        let seg2_data = make_segment_data("test", vec![("doc2", "hello")]);
        Segment::write_segment(dir.path(), 2, &seg2_data).unwrap();

        let results = SegmentSearcher::search_merged(dir.path(), "hello").unwrap();
        assert_eq!(results.len(), 2);
        // doc2 is shorter, should score higher
        assert_eq!(results[0].document_id, "doc2");
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn search_merged_with_no_matches() {
        let dir = tempfile::tempdir().unwrap();

        let seg1_data = make_segment_data("test", vec![("doc1", "hello world")]);
        Segment::write_segment(dir.path(), 1, &seg1_data).unwrap();

        let results = SegmentSearcher::search_merged(dir.path(), "nonexistent").unwrap();
        assert!(results.is_empty());
    }
}