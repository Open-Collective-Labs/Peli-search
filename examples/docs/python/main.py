from pelisearch import PeliSearch, MatchQuery, SearchRequest

client = PeliSearch("http://localhost:7700")
INDEX = "docs"

pages = [
    {"id": "install", "fields": {"title": "Installation", "section": "getting-started", "content": "Install PeliSearch from source or binary."}},
    {"id": "quickstart", "fields": {"title": "Quick Start", "section": "getting-started", "content": "Create an index and run your first search."}},
    {"id": "filtering", "fields": {"title": "Filtering", "section": "guides", "content": "Filter results with field expressions."}},
]

client.create_index(INDEX)
client.bulk_add_documents(INDEX, pages)

results = client.search(
    INDEX,
    SearchRequest(
        query=MatchQuery(match={"content": "index"}),
        filter="section = getting-started",
    ),
)

print("Getting started pages mentioning 'index':")
for hit in results.hits:
    print(f"- {hit.fields['title']}")

client.delete_index(INDEX)
