# Python SDK

## Installation

```bash
pip install pelisearch
```

From this repository:

```bash
pip install -e sdk/python
```

Requires Python 3.10+.

## Quick Start

```python
from pelisearch import PeliSearch, SearchRequest

client = PeliSearch("http://localhost:7700")

client.create_index("products")
client.add_document("products", "p1", {"title": "Widget", "price": 9.99})

results = client.search("products", SearchRequest(q="widget"))
print(results.hits)
```

You can also use `PeliSearchClient` with host/port:

```python
from pelisearch import PeliSearchClient

client = PeliSearchClient(host="127.0.0.1", port=7700)
```

## Searching

```python
from pelisearch import SearchRequest, MatchQuery

# Simple query
client.search("products", SearchRequest(q="wireless mouse"))

# DSL match
client.search("products", SearchRequest(query=MatchQuery(match={"title": "keyboard"})))

# Pagination
client.search("products", SearchRequest(q="shoes", from_=20, size=20))
```

## Filtering

```python
client.search(
    "products",
    SearchRequest(
        q="keyboard",
        filters=[MatchQuery(match={"category": "electronics"})],
    ),
)
```

## Aggregations

```python
results = client.search(
    "jobs",
    SearchRequest(q="engineer"),
)

print(results.aggregations)
```

## Error Handling

```python
from pelisearch import PeliSearchError

try:
    client.get_index("missing")
except PeliSearchError as err:
    print(err.status, err)
```

Use a context manager to close the HTTP connection pool:

```python
with PeliSearch("http://localhost:7700") as client:
    client.health()
```
