use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum QueryClause {
    Match(MatchQuery),
    Term(TermQuery),
    Range(RangeQuery),
    Bool(BoolQuery),
    Phrase(PhraseQuery),
    Fuzzy(FuzzyQuery),
    Prefix(PrefixQuery),
    MultiMatch(MultiMatchQuery),
    ConstantScore(ConstantScoreQuery),
    DisMax(DisjunctionMaxQuery),
    #[serde(rename = "MatchAll")]
    MatchAll,
    #[serde(rename = "MatchNone")]
    MatchNone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchQuery {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermQuery {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeQuery {
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gte: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gt: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lte: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lt: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoolQuery {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must: Vec<QueryClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub should: Vec<QueryClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filter: Vec<QueryClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must_not: Vec<QueryClause>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseQuery {
    pub field: String,
    pub value: String,
    #[serde(default)]
    pub slop: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyQuery {
    pub field: String,
    pub value: String,
    #[serde(default = "default_max_edits")]
    pub max_edit_distance: u8,
    #[serde(default)]
    pub prefix_length: u8,
}

fn default_max_edits() -> u8 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixQuery {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiMatchQuery {
    pub fields: Vec<String>,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantScoreQuery {
    pub filter: Box<QueryClause>,
    pub boost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisjunctionMaxQuery {
    pub queries: Vec<QueryClause>,
    #[serde(default)]
    pub tie_breaker: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub index: String,
    pub document_id: String,
    pub score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlighted: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    #[serde(default)]
    pub aggregations: HashMap<String, serde_json::Value>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub document_count: usize,
    pub fields: Vec<FieldInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    #[serde(rename = "field_type")]
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCreatedResponse {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentCreatedResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDocumentResult {
    pub id: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkResponse {
    pub documents: Vec<BulkDocumentResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListIndexesResponse {
    pub indexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortField {
    pub field: String,
    #[serde(default = "default_sort_order")]
    pub order: String,
}

fn default_sort_order() -> String {
    "Asc".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<QueryClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<QueryClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<SortField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregations: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocumentRequest {
    pub id: String,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkAddRequest {
    pub documents: Vec<AddDocumentRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub request_count: u64,
    pub search_count: u64,
    pub total_latency_ns: u64,
    pub document_count: u64,
    pub index_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIndexRequest {
    pub name: String,
}
