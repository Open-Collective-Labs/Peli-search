use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use pelisearch_core::document::Document;
use pelisearch_core::query::{MatchQuery, Query, RangeQuery, SearchRequest};

use crate::handlers::indexes::ErrorResponse;
use crate::state::SharedState;

// ---------------------------------------------------------------------------
// Document CRUD types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AddDocumentRequest {
    pub id: String,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AddDocumentResponse {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkAddRequest {
    pub documents: Vec<BulkDocument>,
}

#[derive(Debug, Deserialize)]
pub struct BulkDocument {
    pub id: String,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct BulkAddResponse {
    pub documents: Vec<BulkDocumentResult>,
}

#[derive(Debug, Serialize)]
pub struct BulkDocumentResult {
    pub id: String,
    pub status: String,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Search types
// ---------------------------------------------------------------------------

/// Accept both legacy `{"q": "..."}`, the DSL format, and the core Query format.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SearchRequestBody {
    /// Legacy simple query.
    Legacy {
        q: String,
        #[serde(default)]
        from: usize,
        #[serde(default = "default_size")]
        size: usize,
    },
    /// Structured query DSL.
    Dsl(Box<SearchQueryDsl>),
}

/// Structured query DSL body.
///
/// `query` accepts two formats:
/// - Core format: `{"type": "Match", "field": "title", "value": "bike"}`
/// - DSL format: `{"match": {"title": "bike"}}` (backward compatible, supports match/term/range)
#[derive(Debug, Deserialize)]
pub struct SearchQueryDsl {
    pub query: serde_json::Value,
    #[serde(default)]
    pub filters: Vec<serde_json::Value>,
    #[serde(default)]
    pub sort: Vec<pelisearch_core::sort::SortField>,
    #[serde(default)]
    pub aggregations: Vec<pelisearch_core::aggregation::Aggregation>,
    #[serde(default)]
    pub from: usize,
    #[serde(default = "default_size")]
    pub size: usize,
    #[serde(default)]
    pub highlight: bool,
}

fn default_size() -> usize {
    10
}

/// Backward-compatible DSL query clause. Supports `match`, `term`, and `range`.
#[derive(Debug, Deserialize)]
pub struct DslQueryClause {
    #[serde(rename = "match")]
    pub match_: Option<HashMap<String, String>>,
    #[serde(rename = "term")]
    pub term_: Option<HashMap<String, String>>,
    #[serde(rename = "range")]
    pub range_: Option<HashMap<String, RangeCondition>>,
}

#[derive(Debug, Deserialize)]
pub struct RangeCondition {
    pub gte: Option<f64>,
    pub lte: Option<f64>,
    pub gt: Option<f64>,
    pub lt: Option<f64>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /indexes/:name/documents
pub async fn add_document(
    State(state): State<SharedState>,
    Path(index_name): Path<String>,
    Json(payload): Json<AddDocumentRequest>,
) -> Result<(StatusCode, Json<AddDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let doc = Document::new(payload.id.clone(), payload.fields).map_err(|e| {
        bad_request_error(format!("invalid document: {e}"))
    })?;

    let mut engine = state.engine.write().await;
    engine
        .add_document(&index_name, doc)
        .map_err(handle_add_error)?;

    Ok((
        StatusCode::CREATED,
        Json(AddDocumentResponse { id: payload.id }),
    ))
}

/// POST /indexes/:name/documents/bulk
pub async fn bulk_add_documents(
    State(state): State<SharedState>,
    Path(index_name): Path<String>,
    Json(payload): Json<BulkAddRequest>,
) -> Result<(StatusCode, Json<BulkAddResponse>), (StatusCode, Json<ErrorResponse>)> {
    if payload.documents.is_empty() {
        return Err(bad_request_error("documents list must not be empty".into()));
    }

    let mut engine = state.engine.write().await;
    let mut results = Vec::with_capacity(payload.documents.len());

    for doc_req in payload.documents {
        let id = doc_req.id.clone();
        match Document::new(doc_req.id, doc_req.fields) {
            Ok(doc) => match engine.add_document_no_flush(&index_name, doc) {
                Ok(()) => results.push(BulkDocumentResult {
                    id,
                    status: "created".into(),
                    error: None,
                }),
                Err(e) => results.push(BulkDocumentResult {
                    id,
                    status: "error".into(),
                    error: Some(e.to_string()),
                }),
            },
            Err(e) => results.push(BulkDocumentResult {
                id,
                status: "error".into(),
                error: Some(format!("invalid document: {e}")),
            }),
        }
    }

    // Single WAL flush for the entire batch
    let _ = engine.flush_wal();

    Ok((StatusCode::CREATED, Json(BulkAddResponse { documents: results })))
}

/// GET /indexes/:name/documents/:id
pub async fn get_document(
    State(state): State<SharedState>,
    Path((index_name, doc_id)): Path<(String, String)>,
) -> Result<Json<Document>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.read().await;
    let doc = engine
        .get_document(&index_name, &doc_id)
        .map_err(|e| not_found_error(e.to_string()))?
        .clone();
    Ok(Json(doc))
}

/// DELETE /indexes/:name/documents/:id
pub async fn delete_document(
    State(state): State<SharedState>,
    Path((index_name, doc_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut engine = state.engine.write().await;
    engine
        .remove_document(&index_name, &doc_id)
        .map_err(|e| not_found_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /indexes/:name/search
///
/// Accepts three request formats:
///
/// **Legacy** (backward compatible):
/// ```json
/// {"q": "search text"}
/// ```
///
/// **Core Query format** (full support for all query types):
/// ```json
/// {"query": {"type": "Match", "field": "title", "value": "search text"}}
/// ```
///
/// **DSL format** (backward compatible, match/term/range only):
/// ```json
/// {"query": {"match": {"title": "search text"}}}
/// ```
pub async fn search(
    State(state): State<SharedState>,
    Path(index_name): Path<String>,
    Json(body): Json<SearchRequestBody>,
) -> Result<Json<pelisearch_core::types::SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = build_search_request(&body).map_err(bad_request_error)?;

    state.metrics.inc_search();

    let engine = state.engine.read().await;
    let response = engine
        .search_request_with_explanations(&index_name, &request)
        .map_err(|e| not_found_error(e.to_string()))?;
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn build_search_request(body: &SearchRequestBody) -> Result<SearchRequest, String> {
    match body {
        SearchRequestBody::Legacy { q, from, size } => {
            Ok(SearchRequest {
                query: Query::Match(MatchQuery::new("", q)),
                filters: vec![],
                sort: vec![],
                aggregations: vec![],
                from: *from,
                size: *size,
                highlight: false,
            })
        }
        SearchRequestBody::Dsl(dsl) => {
            let query = parse_query_value(&dsl.query)?
                .ok_or_else(|| "a query is required (e.g. \"type\": \"Match\")".to_string())?;

            let mut filters = Vec::new();
            for fv in &dsl.filters {
                if let Some(q) = parse_query_value(fv)? {
                    filters.push(q);
                }
            }

            Ok(SearchRequest {
                query,
                filters,
                sort: dsl.sort.clone(),
                aggregations: dsl.aggregations.clone(),
                from: dsl.from,
                size: dsl.size,
                highlight: dsl.highlight,
            })
        }
    }
}

/// Parse a JSON value as a Query. Tries the core serde format first,
/// then falls back to the backward-compatible DSL format (match/term/range).
fn parse_query_value(value: &serde_json::Value) -> Result<Option<Query>, String> {
    // Try core Query format (uses #[serde(tag = "type")])
    if let Ok(query) = serde_json::from_value::<Query>(value.clone()) {
        return Ok(Some(query));
    }

    // Try backward-compatible DSL format
    if let Ok(dsl) = serde_json::from_value::<DslQueryClause>(value.clone()) {
        let queries = convert_dsl_clause(&dsl);
        let mut results: Vec<Query> = queries.into_iter().collect::<Result<Vec<_>, _>>()?;
        if results.len() == 1 {
            return Ok(Some(results.remove(0)));
        }
        if results.len() > 1 {
            return Err("multiple query clauses in a single query field are not allowed; use {\"type\": \"Bool\"} to combine".to_string());
        }
    }

    Ok(None)
}

fn convert_dsl_clause(qc: &DslQueryClause) -> Vec<Result<Query, String>> {
    let mut queries = Vec::new();

    if let Some(ref fields) = qc.match_ {
        for (field, value) in fields {
            queries.push(Ok(Query::Match(MatchQuery::new(field, value))));
        }
    }

    if let Some(ref fields) = qc.term_ {
        for (field, value) in fields {
            queries.push(Ok(Query::Term(pelisearch_core::query::TermQuery::new(field, value))));
        }
    }

    if let Some(ref fields) = qc.range_ {
        for (field, condition) in fields {
            let mut rq = RangeQuery::new(field);
            if let Some(v) = condition.gte {
                rq = rq.with_gte(v);
            }
            if let Some(v) = condition.lte {
                rq = rq.with_lte(v);
            }
            if let Some(v) = condition.gt {
                rq = rq.with_gt(v);
            }
            if let Some(v) = condition.lt {
                rq = rq.with_lt(v);
            }
            queries.push(Ok(Query::Range(rq)));
        }
    }

    queries
}

fn handle_add_error(e: pelisearch_core::error::SearchError) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        pelisearch_core::error::SearchError::DocumentAlreadyExists(id) => {
            (StatusCode::CONFLICT, Json(ErrorResponse { error: format!("document '{id}' already exists") }))
        }
        _ => bad_request_error(e.to_string())
    }
}

fn bad_request_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: msg }))
}

fn not_found_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: msg }))
}
