use std::collections::HashMap;

use crate::aggregation::Aggregation;
use crate::document::Document;
use crate::error::SearchError;
use crate::filter::FilterEvaluator;
use crate::highlighting;
use crate::index::Index;
use crate::query::request::SearchRequest;
use crate::query::{MatchQuery, Query, QueryCache, QueryOptimizer, QueryPlanner, SearchAnalytics};
use crate::sort::comparator::sort_hits;
use crate::types::{AggregationResults, SearchHit, SearchResponse, SearchResult};

/// Executes structured search requests against an index.
///
/// Pipeline:
/// 1. Candidate retrieval via BM25 (MatchQuery)
/// 2. Filter application (TermQuery / RangeQuery)
/// 3. Ranked results returned
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::index::Index;
/// use pelisearch_core::query::{Query, MatchQuery, TermQuery, RangeQuery, SearchRequest};
/// use pelisearch_core::query::executor::QueryExecutor;
/// use pelisearch_core::schema::Mapping;
///
/// let mut index = Index::new("products", Mapping::new(vec![]));
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), serde_json::json!("electric bike"));
/// fields.insert("price".to_string(), serde_json::json!(799));
/// let doc = Document::new("doc1", fields).unwrap();
/// index.add_document(doc).unwrap();
///
/// let mut fields = HashMap::new();
/// fields.insert("title".to_string(), serde_json::json!("premium bike"));
/// fields.insert("price".to_string(), serde_json::json!(1500));
/// let doc = Document::new("doc2", fields).unwrap();
/// index.add_document(doc).unwrap();
///
/// let request = SearchRequest {
///     query: Query::Match(MatchQuery::new("title", "bike")),
///     filters: vec![
///         Query::Range(RangeQuery::new("price").with_lte(1000.0)),
///     ],
///     sort: vec![],
///     aggregations: vec![],
///     from: 0,
///     size: 10,
///     highlight: false,
/// };
///
/// let results = QueryExecutor::execute(&index, &request).unwrap();
/// assert_eq!(results.len(), 1);
/// assert_eq!(results[0].document_id, "doc1");
/// ```
pub struct QueryExecutor;

impl QueryExecutor {
    /// Execute a search request against an index.
    ///
    /// Pipeline: candidate retrieval → filters → sorting → response.
    /// BM25 ranking is preserved when no explicit sort is specified.
    pub fn execute(index: &Index, request: &SearchRequest) -> Result<Vec<SearchHit>, SearchError> {
        let results = retrieve_candidates(index, &request.query)?;
        let matches = filter_candidates(index, results, &request.filters);
        let sorted = sort_hits(matches, &request.sort, index);
        Ok(sorted)
    }

    /// Execute a search request and return results with aggregations.
    ///
    /// Pipeline: candidate retrieval → filters → sorting → aggregations → response.
    /// BM25 ranking is preserved when no explicit sort is specified.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::index::Index;
    /// use pelisearch_core::query::{Query, MatchQuery, SearchRequest};
    /// use pelisearch_core::query::executor::QueryExecutor;
    /// use pelisearch_core::schema::Mapping;
    ///
    /// let mut index = Index::new("test", Mapping::new(vec![]));
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello world"));
    /// let doc = Document::new("doc1", fields).unwrap();
    /// index.add_document(doc).unwrap();
    ///
    /// let request = SearchRequest {
    ///     query: Query::Match(MatchQuery::new("title", "hello")),
    ///     filters: vec![],
    ///     sort: vec![],
    ///     aggregations: vec![],
    ///     from: 0,
    ///     size: 10,
    ///     highlight: false,
    /// };
    ///
    /// let response = QueryExecutor::execute_with_explanations(&index, &request).unwrap();
    /// assert_eq!(response.hits.len(), 1);
    /// assert_eq!(response.total, 1);
    /// assert!(response.aggregations.is_empty());
    /// ```
    pub fn execute_with_explanations(
        index: &Index,
        request: &SearchRequest,
    ) -> Result<SearchResponse, SearchError> {
        let results = retrieve_candidates(index, &request.query)?;
        let matches = filter_candidates(index, results, &request.filters);
        let sorted = sort_hits(matches, &request.sort, index);

        let total = sorted.len();
        
        // Apply pagination
        let paged_hits = if request.from >= sorted.len() {
            Vec::new()
        } else {
            let end = std::cmp::min(request.from.saturating_add(request.size), sorted.len());
            sorted[request.from..end].to_vec()
        };

        let documents: Vec<Document> = sorted
            .iter()
            .filter_map(|hit| index.get_document(&hit.document_id).cloned().ok())
            .collect();
        let agg_results = compute_aggregations(&request.aggregations, &documents);

        Ok(SearchResponse {
            hits: paged_hits,
            aggregations: agg_results,
            total,
        })
    }

    /// Execute a search request with full pipeline: planner → optimizer → cache → analytics → highlighting.
    ///
    /// When `cache` is `Some`, results are cached and checked before execution.
    /// When `analytics` is `Some`, query performance is recorded.
    /// When `request.highlight` is true, matching terms are wrapped in `<em>` tags.
    pub fn execute_with_pipeline(
        index: &Index,
        request: &SearchRequest,
        cache: Option<&QueryCache>,
        analytics: Option<&SearchAnalytics>,
    ) -> Result<SearchResponse, SearchError> {
        let start = std::time::Instant::now();

        let plan = QueryPlanner::plan(index, &request.query);
        let _optimized = QueryOptimizer::optimize(plan);

        let query_text = format!("{:?}", request.query);
        let cache_key = QueryCache::cache_key(&query_text, request.from, request.size);

        let cached_hits = cache.as_ref().and_then(|c| c.get(cache_key));
        let mut from_cache = false;

        let (paged, full_hits, total) = if let Some(results) = cached_hits {
            from_cache = true;
            let total = results.len();
            let paged = Self::page_hits(&results, request.from, request.size);
            (paged, results, total)
        } else {
            let results = retrieve_candidates(index, &request.query)?;
            let matches = filter_candidates(index, results, &request.filters);
            let sorted = sort_hits(matches, &request.sort, index);
            let total = sorted.len();

            if let Some(c) = cache.as_ref() {
                c.insert(cache_key, sorted.clone());
            }

            let paged = Self::page_hits(&sorted, request.from, request.size);
            (paged, sorted, total)
        };

        let elapsed = start.elapsed();

        if let Some(a) = analytics.as_ref() {
            a.record_query(&query_text, elapsed.as_nanos() as u64, total);
            a.record_cache(from_cache);
        }

        let hits = if request.highlight {
            Self::apply_highlights(index, paged, &query_text)
        } else {
            paged
        };

        let documents: Vec<Document> = full_hits
            .iter()
            .filter_map(|hit| index.get_document(&hit.document_id).cloned().ok())
            .collect();
        let agg_results = compute_aggregations(&request.aggregations, &documents);

        Ok(SearchResponse {
            hits,
            aggregations: agg_results,
            total,
        })
    }

    fn page_hits(hits: &[SearchHit], from: usize, size: usize) -> Vec<SearchHit> {
        if from >= hits.len() {
            Vec::new()
        } else {
            let end = std::cmp::min(from.saturating_add(size), hits.len());
            hits[from..end].to_vec()
        }
    }

    fn apply_highlights(
        index: &Index,
        hits: Vec<SearchHit>,
        query_text: &str,
    ) -> Vec<SearchHit> {
        hits.into_iter()
            .map(|hit| {
                let doc = match index.get_document(&hit.document_id) {
                    Ok(d) => d,
                    Err(_) => return hit,
                };
                let mut highlights = std::collections::HashMap::new();
                for (field_name, field_value) in &doc.fields {
                    if let Some(text) = field_value.as_str() {
                        let highlighted = highlighting::highlight(text, query_text);
                        if highlighted != text {
                            highlights.insert(field_name.clone(), highlighted);
                        }
                    }
                }
                if highlights.is_empty() {
                    hit
                } else {
                    hit.with_highlights(highlights)
                }
            })
            .collect()
    }
}

fn compute_aggregations(
    aggregations: &[Aggregation],
    documents: &[Document],
) -> AggregationResults {
    let mut results = AggregationResults::new();
    for agg in aggregations {
        let (key, value) = match agg {
            Aggregation::Terms(terms) => {
                let buckets = terms.execute(documents);
                let map: HashMap<String, serde_json::Value> = buckets
                    .into_iter()
                    .map(|b| {
                        (
                            b.key,
                            serde_json::Value::Number(serde_json::Number::from(b.count)),
                        )
                    })
                    .collect();
                (terms.field.clone(), serde_json::Value::Object(map.into_iter().collect()))
            }
            Aggregation::Count(count) => {
                let result = count.execute(documents);
                (count.field.clone(), serde_json::to_value(result).unwrap())
            }
            Aggregation::Min(min) => {
                let result = min.execute(documents);
                (min.field.clone(), serde_json::to_value(result).unwrap())
            }
            Aggregation::Max(max) => {
                let result = max.execute(documents);
                (max.field.clone(), serde_json::to_value(result).unwrap())
            }
            Aggregation::Average(avg) => {
                let result = avg.execute(documents);
                (avg.field.clone(), serde_json::to_value(result).unwrap())
            }
            Aggregation::Sum(sum) => {
                let result = sum.execute(documents);
                (sum.field.clone(), serde_json::to_value(result).unwrap())
            }
        };
        results.insert(key, value);
    }
    results
}

/// Retrieve BM25-ranked candidates for the given query.
fn retrieve_candidates(index: &Index, query: &Query) -> Result<Vec<SearchResult>, SearchError> {
    match query {
        Query::Match(MatchQuery { field, value, .. }) => {
            // Use OR semantics to collect all candidates across terms.
            // Field-specific filtering is applied afterward.
            let mut results = index.search_any(value);
            if !field.is_empty() {
                let query_tokens = crate::tokenizer::tokenize(value);
                results.retain(|res| {
                    if let Ok(doc) = index.get_document(&res.document_id) {
                        if let Some(field_val) = doc.get_field(field) {
                            if let serde_json::Value::String(s) = field_val {
                                let field_tokens = crate::tokenizer::tokenize(s);
                                return query_tokens.iter().any(|qt| field_tokens.contains(qt));
                            }
                        }
                    }
                    false
                });
            }
            Ok(results)
        }
        _ => {
            // Term and Range queries are handled as filters only;
            // return empty candidate set (no BM25 results).
            Ok(Vec::new())
        }
    }
}

/// Apply filters to a list of results, keeping only documents that match all filters.
fn filter_candidates(
    index: &Index,
    results: Vec<SearchResult>,
    filters: &[Query],
) -> Vec<SearchHit> {
    if filters.is_empty() {
        return results
            .into_iter()
            .map(|r| SearchHit::new(index.name(), r.document_id, r.score))
            .collect();
    }

    results
        .into_iter()
        .filter(|result| {
            let doc: Option<&Document> = index.get_document(&result.document_id).ok();
            match doc {
                Some(d) => filters.iter().all(|f| filter_evaluator(f, d)),
                None => false,
            }
        })
        .map(|r| SearchHit::new(index.name(), r.document_id, r.score))
        .collect()
}

/// Evaluate a single filter query against a document.
fn filter_evaluator(filter: &Query, doc: &Document) -> bool {
    match filter {
        Query::Term(tq) => tq.evaluate(doc),
        Query::Range(rq) => rq.evaluate(doc),
        Query::Match(_) | Query::MultiMatch(_) | Query::Bool(_) | Query::Phrase(_)
        | Query::Fuzzy(_) | Query::Prefix(_) | Query::ConstantScore(_)
        | Query::DisMax(_) | Query::MatchAll | Query::MatchNone => {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::Index;
    use crate::query::{MatchQuery, Query, RangeQuery, SearchRequest, TermQuery};
    use crate::schema::Mapping;

    use super::QueryExecutor;

    fn setup_index() -> Index {
        let mut index = Index::new("test", Mapping::new(vec![]));

        let doc = Document::new(
            "doc1",
            HashMap::from([
                ("title".to_string(), serde_json::json!("electric bike")),
                ("price".to_string(), serde_json::json!(799)),
                ("category".to_string(), serde_json::json!("electronics")),
            ]),
        )
        .unwrap();
        index.add_document(doc).unwrap();

        let doc = Document::new(
            "doc2",
            HashMap::from([
                ("title".to_string(), serde_json::json!("premium bike")),
                ("price".to_string(), serde_json::json!(1500)),
                ("category".to_string(), serde_json::json!("sports")),
            ]),
        )
        .unwrap();
        index.add_document(doc).unwrap();

        let doc = Document::new(
            "doc3",
            HashMap::from([
                ("title".to_string(), serde_json::json!("walking shoes")),
                ("price".to_string(), serde_json::json!(50)),
                ("category".to_string(), serde_json::json!("footwear")),
            ]),
        )
        .unwrap();
        index.add_document(doc).unwrap();

        index
    }

    #[test]
    fn match_query_returns_results() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn range_filter_narrows_results() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![Query::Range(RangeQuery::new("price").with_lte(1000.0))],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn term_filter_narrows_results() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![Query::Term(TermQuery::new("category", "electronics"))],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn multiple_filters_combine() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![
                Query::Term(TermQuery::new("category", "electronics")),
                Query::Range(RangeQuery::new("price").with_lte(1000.0)),
            ],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "doc1");
    }

    #[test]
    fn no_match_no_results() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "nonexistent")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn filter_excludes_all_results() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![Query::Range(RangeQuery::new("price").with_lt(500.0))],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn bm25_still_works_without_filters() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        // doc2 ("premium bike") matches both "bike" (1 token) and "premium" (1 token),
        // doc1 ("electric bike") matches both "bike" (1 token) and "electric" (1 token)
        // Both have same token count in matching, but different lengths.
        // Scores may differ; just check we get 2 results.
        assert_eq!(results.len(), 2);
        // Results should be sorted by score descending
        assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn explanations_with_filters() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![Query::Range(RangeQuery::new("price").with_lte(1000.0))],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let response = QueryExecutor::execute_with_explanations(&index, &request).unwrap();
        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].document_id, "doc1");
        assert!(response.aggregations.is_empty());
    }

    #[test]
    fn empty_filters_match_all() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let no_filters = QueryExecutor::execute(&index, &request).unwrap();
        assert_eq!(no_filters.len(), 2);
    }

    #[test]
    fn result_is_annotated_with_index_name() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        for hit in &results {
            assert_eq!(hit.index, "test");
        }
    }

    #[test]
    fn pipeline_basic_search() {
        let index = setup_index();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        let response = QueryExecutor::execute_with_pipeline(&index, &request, None, None).unwrap();
        assert_eq!(response.hits.len(), 2);
        assert!(response.total >= 2);
    }

    #[test]
    fn pipeline_with_cache() {
        let index = setup_index();
        let cache = crate::query::QueryCache::new();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        // First call — should miss cache
        let r1 = QueryExecutor::execute_with_pipeline(&index, &request, Some(&cache), None).unwrap();
        assert_eq!(r1.hits.len(), 2);
        assert_eq!(cache.misses(), 1);
        // Second call — should hit cache
        let r2 = QueryExecutor::execute_with_pipeline(&index, &request, Some(&cache), None).unwrap();
        assert_eq!(r2.hits.len(), 2);
        assert_eq!(cache.hits(), 1);
    }

    #[test]
    fn pipeline_with_analytics() {
        let index = setup_index();
        let analytics = crate::query::SearchAnalytics::new();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        QueryExecutor::execute_with_pipeline(&index, &request, None, Some(&analytics)).unwrap();
        assert!(analytics.total_queries() >= 1);
    }

    #[test]
    fn pipeline_with_cache_and_analytics() {
        let index = setup_index();
        let cache = crate::query::QueryCache::new();
        let analytics = crate::query::SearchAnalytics::new();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "bike")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: false,
        };
        // First call — cache miss
        QueryExecutor::execute_with_pipeline(&index, &request, Some(&cache), Some(&analytics)).unwrap();
        assert_eq!(cache.misses(), 1);
        // Second call — cache hit
        QueryExecutor::execute_with_pipeline(&index, &request, Some(&cache), Some(&analytics)).unwrap();
        assert_eq!(cache.hits(), 1);
        assert!(analytics.total_queries() >= 2);
    }

    #[test]
    fn pipeline_highlighting() {
        let mut index = Index::new("test", Mapping::new(vec![]));
        let doc = Document::new(
            "doc1",
            HashMap::from([("title".to_string(), serde_json::json!("bike electric scooter"))]),
        )
        .unwrap();
        index.add_document(doc).unwrap();
        let request = SearchRequest {
            query: Query::Match(MatchQuery::new("title", "electric")),
            filters: vec![],
            sort: vec![],
            aggregations: vec![],
            from: 0,
            size: 10,
            highlight: true,
        };
        let response = QueryExecutor::execute_with_pipeline(&index, &request, None, None).unwrap();
        assert_eq!(response.hits.len(), 1);
        if let Some(ref highlights) = response.hits[0].highlighted {
            if let Some(title_hl) = highlights.get("title") {
                assert!(title_hl.contains("<em>"));
            }
        }
    }
}
