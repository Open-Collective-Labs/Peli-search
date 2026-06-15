use std::collections::HashMap;

use crate::aggregation::Aggregation;
use crate::document::Document;
use crate::error::SearchError;
use crate::filter::FilterEvaluator;
use crate::index::Index;
use crate::query::request::SearchRequest;
use crate::query::{MatchQuery, Query};
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
    /// };
    ///
    /// let response = QueryExecutor::execute_with_explanations(&index, &request).unwrap();
    /// assert_eq!(response.hits.len(), 1);
    /// assert!(response.aggregations.is_empty());
    /// ```
    pub fn execute_with_explanations(
        index: &Index,
        request: &SearchRequest,
    ) -> Result<SearchResponse, SearchError> {
        let results = retrieve_candidates(index, &request.query)?;
        let matches = filter_candidates(index, results, &request.filters);
        let sorted = sort_hits(matches, &request.sort, index);

        let documents: Vec<Document> = sorted
            .iter()
            .filter_map(|hit| index.get_document(&hit.document_id).cloned().ok())
            .collect();
        let agg_results = compute_aggregations(&request.aggregations, &documents);

        Ok(SearchResponse {
            hits: sorted,
            aggregations: agg_results,
        })
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
        Query::Match(MatchQuery { value, .. }) => Ok(index.search(value)),
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
        Query::Match(_) => {
            // Match queries are not used as filters; skip.
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
        };
        let results = QueryExecutor::execute(&index, &request).unwrap();
        for hit in &results {
            assert_eq!(hit.index, "test");
        }
    }
}
