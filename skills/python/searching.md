# Python Search Examples

```python
from pelisearch import SearchRequest, MatchQuery, SortField, RangeQuery, RangeCondition

# Simple query
results = client.search("articles", SearchRequest(q="rust"))
print(f"Found {results.total} results")

# Field match with highlighting
results = client.search("articles", SearchRequest(
    query=MatchQuery(match={"title": "rust"}),
    highlight=True,
))
for hit in results.hits:
    if hit.highlighted:
        print(hit.highlighted.get("title"))

# Filtered search
results = client.search("products", SearchRequest(
    query=MatchQuery(match={"name": "keyboard"}),
    filters=[RangeQuery(range={"price": RangeCondition(gte=50, lte=200)})],
))

# Paginated and sorted
results = client.search("products", SearchRequest(
    query=MatchQuery(match={"category": "electronics"}),
    sort=[SortField(field="price", order="Asc")],
    from_=0,
    size=20,
))
```
