package pelisearch

// Types shared across the Go SDK.

// SearchHit represents a single search result.
type SearchHit struct {
	Index       string                 `json:"index,omitempty"`
	DocumentID  string                 `json:"document_id"`
	Score       float64                `json:"score"`
	Fields      map[string]interface{} `json:"fields,omitempty"`
	Highlights  map[string][]string    `json:"highlights,omitempty"`
}

// SearchResponse is the full search response.
type SearchResponse struct {
	Hits               []SearchHit               `json:"hits"`
	Aggregations       map[string]interface{}    `json:"aggregations,omitempty"`
	TotalHits          *int64                    `json:"total_hits,omitempty"`
	Page               *int                      `json:"page,omitempty"`
	PageSize           *int                      `json:"page_size,omitempty"`
	FacetDistributions map[string]map[string]int `json:"facet_distributions,omitempty"`
}

// IndexInfo describes an index.
type IndexInfo struct {
	Name          string           `json:"name"`
	DocumentCount int              `json:"document_count"`
	Fields        []FieldInfo      `json:"fields"`
}

// FieldInfo describes a single field in the schema.
type FieldInfo struct {
	Name string `json:"name"`
	Type string `json:"type"`
}

// IndexCreatedResponse is returned after creating an index.
type IndexCreatedResponse struct {
	Name string `json:"name"`
}

// DocumentCreatedResponse is returned after adding a document.
type DocumentCreatedResponse struct {
	ID string `json:"id"`
}

// BulkDocumentResult is the result for one document in a bulk operation.
type BulkDocumentResult struct {
	ID     string  `json:"id"`
	Status string  `json:"status"`
	Error  *string `json:"error"`
}

// BulkResponse is returned from a bulk add operation.
type BulkResponse struct {
	Documents []BulkDocumentResult `json:"documents"`
}

// ErrorResponse is a standard error payload.
type ErrorResponse struct {
	Error string `json:"error"`
}

// SearchRequest represents an incoming search request.
type SearchRequest struct {
	Q                *string                `json:"q,omitempty"`
	Query            map[string]interface{} `json:"query,omitempty"`
	Filter           *string                `json:"filter,omitempty"`
	Sort             []string               `json:"sort,omitempty"`
	Page             *int                   `json:"page,omitempty"`
	PageSize         *int                   `json:"page_size,omitempty"`
	Facets           []string               `json:"facets,omitempty"`
	Highlight        *bool                  `json:"highlight,omitempty"`
	HighlightFields  []string               `json:"highlight_fields,omitempty"`
	HighlightPreTag  *string                `json:"highlight_pre_tag,omitempty"`
	HighlightPostTag *string                `json:"highlight_post_tag,omitempty"`
}

// MatchQuery is a match query clause.
type MatchQuery struct {
	Match map[string]string `json:"match"`
}

// TermQuery is a term query clause.
type TermQuery struct {
	Term map[string]string `json:"term"`
}

// RangeCondition defines a range filter.
type RangeCondition struct {
	Gte *float64 `json:"gte,omitempty"`
	Lte *float64 `json:"lte,omitempty"`
	Gt  *float64 `json:"gt,omitempty"`
	Lt  *float64 `json:"lt,omitempty"`
}

// RangeQuery is a range query clause.
type RangeQuery struct {
	Range map[string]RangeCondition `json:"range"`
}
