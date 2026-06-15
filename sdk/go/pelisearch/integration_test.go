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

	filter := "category = electronics"
	results, err = client.Search(ctx, index, &pelisearch.SearchRequest{
		Query: map[string]interface{}{"match": map[string]string{"title": "keyboard"}},
		Filter: &filter,
	})
	if err != nil {
		t.Fatalf("dsl search: %v", err)
	}
	if len(results.Hits) == 0 {
		t.Fatal("expected dsl search hits")
	}

	if err := client.DeleteIndex(ctx, index); err != nil {
		t.Fatalf("delete index: %v", err)
	}
}

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
