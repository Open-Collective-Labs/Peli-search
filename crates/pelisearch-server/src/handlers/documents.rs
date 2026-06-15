use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use pelisearch_core::document::Document;
use pelisearch_core::query::{MatchQuery, Query, SearchRequest};

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
// Search DSL types
// ---------------------------------------------------------------------------

/// Accept both legacy `{"q": "..."}` and the new query DSL.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SearchRequestBody {
    /// Legacy simple query.
    Legacy { q: String },
    /// Structured query DSL.
    Dsl(SearchQueryDsl),
}

/// Structured query DSL body.
#[derive(Debug, Deserialize)]
pub struct SearchQueryDsl {
    pub query: QueryClause,
}

/// A single query clause. Currently only `match` is supported.
#[derive(Debug, Deserialize)]
pub struct QueryClause {
    /// Full-text match: `{"match": {"field_name": "search text"}}`
    #[serde(rename = "match")]
    pub match_: Option<HashMap<String, String>>,
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

    let mut engine = state.engine.lock().await;
    engine
        .add_document(&index_name, doc)
        .map_err(|e| handle_add_error(e))?;

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

    let mut engine = state.engine.lock().await;
    let mut results = Vec::with_capacity(payload.documents.len());

    for doc_req in payload.documents {
        let id = doc_req.id.clone();
        match Document::new(doc_req.id, doc_req.fields) {
            Ok(doc) => match engine.add_document(&index_name, doc) {
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

    Ok((StatusCode::CREATED, Json(BulkAddResponse { documents: results })))
}

/// GET /indexes/:name/documents/:id
pub async fn get_document(
    State(state): State<SharedState>,
    Path((index_name, doc_id)): Path<(String, String)>,
) -> Result<Json<Document>, (StatusCode, Json<ErrorResponse>)> {
    let engine = state.engine.lock().await;
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
    let mut engine = state.engine.lock().await;
    engine
        .remove_document(&index_name, &doc_id)
        .map_err(|e| not_found_error(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /indexes/:name/search
///
/// Accepts two request formats:
///
/// **Legacy** (backward compatible):
/// ```json
/// {"q": "search text"}
/// ```
///
/// **Query DSL**:
/// ```json
/// {"query": {"match": {"field": "search text"}}}
/// ```
pub async fn search(
    State(state): State<SharedState>,
    Path(index_name): Path<String>,
    Json(body): Json<SearchRequestBody>,
) -> Result<Json<pelisearch_core::types::SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request = build_search_request(&body).map_err(|e| bad_request_error(e))?;

    state.metrics.inc_search();

    let engine = state.engine.lock().await;
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
        SearchRequestBody::Legacy { q } => {
            // Convert legacy `{"q": "..."}` to a match-all-fields query.
            Ok(SearchRequest {
                query: Query::Match(MatchQuery::new("", q)),
                filters: vec![],
                sort: vec![],
                aggregations: vec![],
            })
        }
        SearchRequestBody::Dsl(dsl) => {
            let mut queries: Vec<Query> = Vec::new();

            if let Some(ref match_fields) = dsl.query.match_ {
                for (field, value) in match_fields {
                    queries.push(Query::Match(MatchQuery::new(field, value)));
                }
            }

            let query = queries.pop().ok_or_else(|| {
                "at least one query clause is required (e.g. \"match\")".to_string()
            })?;

            Ok(SearchRequest {
                query,
                filters: vec![],
                sort: vec![],
                aggregations: vec![],
            })
        }
    }
}

fn handle_add_error(e: pelisearch_core::error::SearchError) -> (StatusCode, Json<ErrorResponse>) {
    let msg = e.to_string();
    if msg.contains("already exists") {
        (StatusCode::CONFLICT, Json(ErrorResponse { error: msg }))
    } else {
        bad_request_error(msg)
    }
}

fn bad_request_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: msg }))
}

fn not_found_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: msg }))
}
