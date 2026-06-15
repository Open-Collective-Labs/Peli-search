package pelisearch

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
)

// Client is the PeliSearch HTTP client.
type Client struct {
	baseURL    string
	httpClient *http.Client
	headers    map[string]string
}

// NewClient creates a new PeliSearch client.
// Pass a full base URL (e.g. "http://localhost:7700") or host and port separately.
func NewClient(host string, port int, opts ...ClientOption) *Client {
	baseURL := host
	if !hasScheme(host) {
		baseURL = fmt.Sprintf("http://%s:%d", host, port)
	}
	c := &Client{
		baseURL:    baseURL,
		httpClient: &http.Client{},
		headers: map[string]string{
			"Content-Type": "application/json",
		},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// NewClientFromURL creates a client from a full base URL.
func NewClientFromURL(baseURL string, opts ...ClientOption) *Client {
	c := &Client{
		baseURL:    baseURL,
		httpClient: &http.Client{},
		headers: map[string]string{
			"Content-Type": "application/json",
		},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// ClientOption configures a Client.
type ClientOption func(*Client)

// WithHTTPClient sets a custom HTTP client.
func WithHTTPClient(httpClient *http.Client) ClientOption {
	return func(c *Client) {
		c.httpClient = httpClient
	}
}

// WithAPIKey sets the API key header.
func WithAPIKey(key string) ClientOption {
	return func(c *Client) {
		c.headers["X-Api-Key"] = key
	}
}

// ── Health ────────────────────────────────────────────────────────

// Health checks server health.
func (c *Client) Health(ctx context.Context) error {
	_, err := c.do(ctx, "GET", "/health", nil)
	return err
}

// Ready checks server readiness.
func (c *Client) Ready(ctx context.Context) error {
	_, err := c.do(ctx, "GET", "/ready", nil)
	return err
}

// ── Internal ──────────────────────────────────────────────────────

func (c *Client) do(ctx context.Context, method, path string, body interface{}) ([]byte, error) {
	var buf bytes.Buffer
	if body != nil {
		if err := json.NewEncoder(&buf).Encode(body); err != nil {
			return nil, fmt.Errorf("encode body: %w", err)
		}
	}

	req, err := http.NewRequestWithContext(ctx, method, c.baseURL+path, &buf)
	if err != nil {
		return nil, fmt.Errorf("create request: %w", err)
	}
	for k, v := range c.headers {
		req.Header.Set(k, v)
	}

	res, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("do request: %w", err)
	}
	defer res.Body.Close()

	data, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("read body: %w", err)
	}

	if res.StatusCode >= 400 {
		var errResp ErrorResponse
		if json.Unmarshal(data, &errResp) == nil && errResp.Error != "" {
			return nil, &APIError{Status: res.StatusCode, Message: errResp.Error}
		}
		return nil, &APIError{Status: res.StatusCode, Message: strconv.Itoa(res.StatusCode)}
	}

	return data, nil
}

func (c *Client) doInto(ctx context.Context, method, path string, body, target interface{}) error {
	data, err := c.do(ctx, method, path, body)
	if err != nil {
		return err
	}
	if len(data) == 0 {
		return nil
	}
	return json.Unmarshal(data, target)
}

func hasScheme(host string) bool {
	return strings.HasPrefix(host, "http://") || strings.HasPrefix(host, "https://")
}

// APIError is returned when the server responds with an error status.
type APIError struct {
	Status  int
	Message string
}

func (e *APIError) Error() string {
	return fmt.Sprintf("pelisearch error (%d): %s", e.Status, e.Message)
}
