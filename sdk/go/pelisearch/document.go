package pelisearch

import (
	"context"
	"fmt"
	"net/url"
)

// AddDocument adds a document to an index.
func (c *Client) AddDocument(ctx context.Context, index, id string, fields map[string]interface{}) (*DocumentCreatedResponse, error) {
	body := map[string]interface{}{
		"id":     id,
		"fields": fields,
	}
	var resp DocumentCreatedResponse
	if err := c.doInto(ctx, "POST", fmt.Sprintf("/indexes/%s/documents", url.PathEscape(index)), body, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetDocument retrieves a document by ID.
func (c *Client) GetDocument(ctx context.Context, index, id string) (map[string]interface{}, error) {
	var resp map[string]interface{}
	if err := c.doInto(ctx, "GET", fmt.Sprintf("/indexes/%s/documents/%s", url.PathEscape(index), url.PathEscape(id)), nil, &resp); err != nil {
		return nil, err
	}
	return resp, nil
}

// DeleteDocument deletes a document by ID.
func (c *Client) DeleteDocument(ctx context.Context, index, id string) error {
	_, err := c.do(ctx, "DELETE", fmt.Sprintf("/indexes/%s/documents/%s", url.PathEscape(index), url.PathEscape(id)), nil)
	return err
}

// BulkAddDocuments adds multiple documents in one request.
func (c *Client) BulkAddDocuments(ctx context.Context, index string, docs []map[string]interface{}) (*BulkResponse, error) {
	body := map[string]interface{}{
		"documents": docs,
	}
	var resp BulkResponse
	if err := c.doInto(ctx, "POST", fmt.Sprintf("/indexes/%s/documents/bulk", url.PathEscape(index)), body, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}
