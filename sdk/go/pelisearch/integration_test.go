package pelisearch_test

import (
	"context"
	"os"
	"testing"
	"time"

	"github.com/Open-Collective-Labs/Peli-search/sdk/go/pelisearch"
)

func testURL() string {
	if u := os.Getenv("PELISEARCH_TEST_URL"); u != "" {
		return u
	}
	return "http://127.0.0.1:7700"
}

func TestIntegration(t *testing.T) {
	client := pelisearch.NewClientFromURL(testURL())
	ctx := context.Background()
	const index = "sdk_go_test"

	_ = client.DeleteIndex(ctx, index)

	if err := client.Health(ctx); err != nil {
		t.Fatalf("health: %v", err)
	}

	if _, err := client.CreateIndex(ctx, index); err != nil {
		t.Fatalf("create index: %v", err)
	}

	indexes, err := client.ListIndexes(ctx)
	if err != nil {
		t.Fatalf("list indexes: %v", err)
	}
	found := false
	for _, name := range indexes {
		if name == index {
			found = true
			break
		}
	}
	if !found {
		t.Fatalf("index %q not in list", index)
	}

	if _, err := client.AddDocument(ctx, index, "d1", map[string]interface{}{
		"title":    "Wireless Mouse",
		"category": "electronics",
		"price":    29.99,
	}); err != nil {
		t.Fatalf("add document: %v", err)
	}

	if _, err := client.BulkAddDocuments(ctx, index, []map[string]interface{}{
		{"id": "d2", "fields": map[string]interface{}{"title": "Keyboard", "category": "electronics"}},
	}); err != nil {
		t.Fatalf("bulk add: %v", err)
	}

	q := "mouse"
	results, err := client.Search(ctx, index, &pelisearch.SearchRequest{Q: &q})
	if err != nil {
		t.Fatalf("search: %v", err)
	}
	if len(results.Hits) == 0 {
		t.Fatal("expected search hits")
	}
	if results.Total < len(results.Hits) {
		t.Fatal("total should be at least hits length")
	}
	for _, hit := range results.Hits {
		if hit.Index == "" {
			t.Fatal("hit missing index")
		}
	}

	results, err = client.Search(ctx, index, &pelisearch.SearchRequest{
		Query: map[string]interface{}{"match": map[string]string{"title": "keyboard"}},
	})
	if err != nil {
		t.Fatalf("dsl search: %v", err)
	}
	if len(results.Hits) == 0 {
		t.Fatal("expected dsl search hits")
	}

	results, err = client.Search(ctx, index, &pelisearch.SearchRequest{
		Q:    strPtr("mouse"),
		From: intPtr(0),
		Size: intPtr(1),
	})
	if err != nil {
		t.Fatalf("paged search: %v", err)
	}
	if len(results.Hits) > 1 {
		t.Fatal("expected at most 1 hit with size=1")
	}

	if err := client.DeleteIndex(ctx, index); err != nil {
		t.Fatalf("delete index: %v", err)
	}
}

func strPtr(s string) *string { return &s }

func intPtr(i int) *int { return &i }

func TestContextTimeout(t *testing.T) {
	client := pelisearch.NewClientFromURL(testURL())
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := client.Health(ctx); err != nil {
		t.Fatalf("health with context: %v", err)
	}
}

func TestMissingIndexError(t *testing.T) {
	client := pelisearch.NewClientFromURL(testURL())
	_, err := client.GetIndex(context.Background(), "nonexistent_sdk_index")
	if err == nil {
		t.Fatal("expected error")
	}
	if _, ok := err.(*pelisearch.APIError); !ok {
		t.Fatalf("expected APIError, got %T", err)
	}
}
