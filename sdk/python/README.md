# PeliSearch Python SDK

Official Python client for [PeliSearch](https://github.com/Open-Collective-Labs/Peli-search).

## Installation

```bash
pip install pelisearch
```

Requires Python 3.10+.

## Quick Start

```python
from pelisearch import PeliSearch, SearchRequest

client = PeliSearch("http://localhost:7700")

# Create an index
client.create_index("products")

# Add documents
client.add_document("products", "doc1", {"title": "Widget", "price": 9.99})

# Search
results = client.search("products", SearchRequest(q="widget"))
for hit in results.hits:
    print(hit.document_id, hit.score)
```

## Documentation

See the [full SDK docs](https://github.com/Open-Collective-Labs/Peli-search) for detailed usage.

## License

MIT
