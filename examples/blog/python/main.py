from pelisearch import PeliSearch, SearchRequest

client = PeliSearch("http://localhost:7700")
INDEX = "posts"

posts = [
    {"id": "1", "fields": {"title": "Getting Started with PeliSearch", "body": "Learn how to index and search documents."}},
    {"id": "2", "fields": {"title": "Advanced Filtering", "body": "Use filters to narrow search results."}},
    {"id": "3", "fields": {"title": "Building a Blog", "body": "Add search to your content site."}},
]

client.create_index(INDEX)
client.bulk_add_documents(INDEX, posts)

results = client.search(
    INDEX,
    SearchRequest(q="filter", highlight=True, highlight_fields=["title", "body"]),
)

for hit in results.hits:
    print(hit.document_id, hit.fields["title"])
    if hit.highlights:
        print("  highlights:", hit.highlights)

client.delete_index(INDEX)
