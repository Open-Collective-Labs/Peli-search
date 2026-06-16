package pelisearch

import (
	"context"
	"fmt"
	"net/url"
)

// Search executes a search request.
func (c *Client) Search(ctx context.Context, index string, req *SearchRequest) (*SearchResponse, error) {
	var resp SearchResponse
	if err := c.doInto(ctx, "POST", fmt.Sprintf("/indexes/%s/search", url.PathEscape(index)), req, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

// Query helpers construct map[string]interface{} values compatible with
// SearchRequest.Query using the core serde format (type-tagged JSON).
// These are safe to use alongside raw map literals.

// MatchQuery returns a match query.
func MatchQuery(field, value string) map[string]interface{} {
	return map[string]interface{}{
		"type":  "Match",
		"field": field,
		"value": value,
	}
}

// TermQuery returns a term query.
func TermQuery(field, value string) map[string]interface{} {
	return map[string]interface{}{
		"type":  "Term",
		"field": field,
		"value": value,
	}
}

// RangeQuery returns a range query using the given RangeCondition.
func RangeQuery(field string, cond RangeCondition) map[string]interface{} {
	m := map[string]interface{}{
		"type":  "Range",
		"field": field,
	}
	if cond.Gte != nil {
		m["gte"] = *cond.Gte
	}
	if cond.Gt != nil {
		m["gt"] = *cond.Gt
	}
	if cond.Lte != nil {
		m["lte"] = *cond.Lte
	}
	if cond.Lt != nil {
		m["lt"] = *cond.Lt
	}
	return m
}

// BoolQuery returns a bool (compound) query. Pass nil for unused clause slices.
func BoolQuery(must, filter, mustNot, should []map[string]interface{}) map[string]interface{} {
	m := map[string]interface{}{
		"type": "Bool",
	}
	if len(must) > 0 {
		m["must"] = must
	}
	if len(filter) > 0 {
		m["filter"] = filter
	}
	if len(mustNot) > 0 {
		m["must_not"] = mustNot
	}
	if len(should) > 0 {
		m["should"] = should
	}
	return m
}

// PhraseQuery returns a phrase query with an optional slop parameter.
func PhraseQuery(field, value string, slop ...int) map[string]interface{} {
	m := map[string]interface{}{
		"type":  "Phrase",
		"field": field,
		"value": value,
	}
	if len(slop) > 0 {
		m["slop"] = slop[0]
	}
	return m
}

// FuzzyQuery returns a fuzzy query with optional maxEditDistance.
func FuzzyQuery(field, value string, maxEditDistance ...uint8) map[string]interface{} {
	m := map[string]interface{}{
		"type":  "Fuzzy",
		"field": field,
		"value": value,
	}
	if len(maxEditDistance) > 0 {
		m["max_edit_distance"] = maxEditDistance[0]
	}
	return m
}

// PrefixQuery returns a prefix query.
func PrefixQuery(field, value string) map[string]interface{} {
	return map[string]interface{}{
		"type":  "Prefix",
		"field": field,
		"value": value,
	}
}

// MatchAll returns a match-all query.
func MatchAll() map[string]interface{} {
	return map[string]interface{}{
		"type": "MatchAll",
	}
}

// MatchNone returns a match-none query.
func MatchNone() map[string]interface{} {
	return map[string]interface{}{
		"type": "MatchNone",
	}
}
