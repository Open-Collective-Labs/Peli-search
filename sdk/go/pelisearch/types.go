package pelisearch

// Types shared across the Go SDK.

// SearchHit represents a single search result.
type SearchHit struct {
	Index       string              `json:"index"`
	DocumentID  string              `json:"document_id"`
	Score       float64             `json:"score"`
	Highlighted map[string]string   `json:"highlighted,omitempty"`
}

// SearchResponse is the full search response.
type SearchResponse struct {
	Hits         []SearchHit            `json:"hits"`
	Aggregations map[string]interface{} `json:"aggregations,omitempty"`
	Total        int                    `json:"total"`
}

// IndexInfo describes an index.
type IndexInfo struct {
	Name          string      `json:"name"`
	DocumentCount int         `json:"document_count"`
	Fields        []FieldInfo `json:"fields"`
}

// FieldInfo describes a single field in the schema.
type FieldInfo struct {
	Name      string `json:"name"`
	FieldType string `json:"field_type"`
	Required  bool   `json:"required"`
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

// SortField defines a single sort specification.
type SortField struct {
	Field string `json:"field"`
	Order string `json:"order,omitempty"`
}

// SearchRequest represents an incoming search request.
type SearchRequest struct {
	Q            *string                `json:"q,omitempty"`
	Query        map[string]interface{} `json:"query,omitempty"`
	Filters      []interface{}          `json:"filters,omitempty"`
	Sort         []SortField            `json:"sort,omitempty"`
	From         *int                   `json:"from,omitempty"`
	Size         *int                   `json:"size,omitempty"`
	Highlight    *bool                  `json:"highlight,omitempty"`
	Aggregations []interface{}          `json:"aggregations,omitempty"`
}

// RangeCondition defines a range filter.
type RangeCondition struct {
	Gte *float64 `json:"gte,omitempty"`
	Lte *float64 `json:"lte,omitempty"`
	Gt  *float64 `json:"gt,omitempty"`
	Lt  *float64 `json:"lt,omitempty"`
}

// CoreQuery represents a query in the core serde format (type-tagged).
// All fields are optional except Type; which fields are used depends on Type.
type CoreQuery struct {
	Type            string       `json:"type"`
	Field           string       `json:"field,omitempty"`
	Value           interface{}  `json:"value,omitempty"`
	Gte             *float64     `json:"gte,omitempty"`
	Gt              *float64     `json:"gt,omitempty"`
	Lte             *float64     `json:"lte,omitempty"`
	Lt              *float64     `json:"lt,omitempty"`
	Must            []CoreQuery  `json:"must,omitempty"`
	Filter          []CoreQuery  `json:"filter,omitempty"`
	MustNot         []CoreQuery  `json:"must_not,omitempty"`
	Should          []CoreQuery  `json:"should,omitempty"`
	Slop            *int         `json:"slop,omitempty"`
	MaxEditDistance *uint8       `json:"max_edit_distance,omitempty"`
	PrefixLength    *uint8       `json:"prefix_length,omitempty"`
}
