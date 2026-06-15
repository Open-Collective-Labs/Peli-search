package pelisearch

import (
	"context"
	"fmt"
	"net/url"
)

// ListIndexes returns all index names.
func (c *Client) ListIndexes(ctx context.Context) ([]string, error) {
	var resp struct {
		Indexes []string `json:"indexes"`
	}
	if err := c.doInto(ctx, "GET", "/indexes", nil, &resp); err != nil {
		return nil, err
	}
	return resp.Indexes, nil
}

// GetIndex returns info about a specific index.
func (c *Client) GetIndex(ctx context.Context, name string) (*IndexInfo, error) {
	var info IndexInfo
	if err := c.doInto(ctx, "GET", fmt.Sprintf("/indexes/%s", url.PathEscape(name)), nil, &info); err != nil {
		return nil, err
	}
	return &info, nil
}

// CreateIndex creates a new index.
func (c *Client) CreateIndex(ctx context.Context, name string) (*IndexCreatedResponse, error) {
	var resp IndexCreatedResponse
	if err := c.doInto(ctx, "POST", "/indexes", map[string]string{"name": name}, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

// DeleteIndex deletes an index.
func (c *Client) DeleteIndex(ctx context.Context, name string) error {
	_, err := c.do(ctx, "DELETE", fmt.Sprintf("/indexes/%s", url.PathEscape(name)), nil)
	return err
}
