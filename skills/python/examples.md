# Python Example Projects

## E-commerce Search

```python
from pelisearch import PeliSearch, SearchRequest, MatchQuery, SortField

client = PeliSearch("http://localhost:7700")

# Index products
client.bulk_add_documents("products", [
    {"id": "mbp", "fields": {"name": "MacBook Pro", "brand": "Apple", "category": "laptops", "price": 1999}},
    {"id": "xps", "fields": {"name": "Dell XPS 13", "brand": "Dell", "category": "laptops", "price": 1499}},
])

# Search with filters and sorting
results = client.search("products", SearchRequest(
    query=MatchQuery(match={"name": "laptop"}),
    filters=[{"term": {"category": "laptops"}}],
    sort=[SortField(field="price", order="Asc")],
    from_=0,
    size=10,
))
```

## Documentation Search

```python
# Index docs
client.bulk_add_documents("docs", [
    {"id": "1", "fields": {"title": "Installation Guide", "content": "...", "section": "getting-started"}},
    {"id": "2", "fields": {"title": "API Reference", "content": "...", "section": "reference"}},
])

# Search with highlighting
results = client.search("docs", SearchRequest(
    query=MatchQuery(match={"content": "installation"}),
    highlight=True,
))
```
