# Python SDK

## Install

```bash
pip install pelisearch
```

## Client

```python
from pelisearch import PeliSearch

client = PeliSearch("http://localhost:7700")
# or: PeliSearch(host="127.0.0.1", port=7700)

# Context manager support:
with PeliSearch("http://localhost:7700") as client:
    client.health()
```

## Indexing

```python
from pelisearch import PeliSearch

client = PeliSearch("http://localhost:7700")

# Create index
client.create_index("products")

# Add document
client.add_document("products", "p1", {
    "name": "Wireless Mouse",
    "category": "electronics",
    "price": 29.99,
})

# Bulk add
client.bulk_add_documents("products", [
    {"id": "p2", "fields": {"name": "Keyboard", "category": "electronics", "price": 89.99}},
    {"id": "p3", "fields": {"name": "Monitor", "category": "electronics", "price": 299.99}},
])

# List indexes
indexes: list[str] = client.list_indexes()

# Get index info
info = client.get_index("products")

# Delete document / index
client.delete_document("products", "p1")
client.delete_index("products")
```

## Searching

```python
from pelisearch import SearchRequest, MatchQuery, SortField

# Simple text search
results = client.search("products", SearchRequest(q="mouse"))
for hit in results.hits:
    print(hit.document_id, hit.score)

# Field-specific match
results = client.search("products", SearchRequest(
    query=MatchQuery(match={"name": "keyboard"}),
))

# With filters, sort, pagination, highlighting
results = client.search("products", SearchRequest(
    query=MatchQuery(match={"name": "keyboard"}),
    filters=[{"term": {"category": "electronics"}}],
    sort=[SortField(field="price", order="Asc")],
    from_=0,
    size=20,
    highlight=True,
))
```

## Response

```python
@dataclass
class SearchResponse:
    hits: list[SearchHit]
    total: int
    aggregations: dict[str, Any]

@dataclass
class SearchHit:
    index: str
    document_id: str
    score: float
    highlighted: Optional[dict[str, str]]
```

## Error Handling

```python
from pelisearch import PeliSearchError

try:
    client.get_index("missing")
except PeliSearchError as err:
    print(err.status, err)
```
