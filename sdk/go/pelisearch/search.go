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
