# Aggregations

Aggregations summarize and group your data. They run alongside search queries and return statistical metrics.

## Terms Aggregation

Group documents by field values and count them:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "aggregations": [
      { "type": "Terms", "field": "brand", "size": 10 }
    ]
  }'
```

Response:

```json
{
  "hits": [...],
  "aggregations": {
    "brand": {
      "apple": 42,
      "dell": 35,
      "lenovo": 28
    }
  }
}
```

## Metrics Aggregation

Compute numeric statistics on a field:

```bash
curl -X POST http://127.0.0.1:7700/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match": { "name": "laptop" } },
    "aggregations": [
      { "type": "Average", "field": "price" },
      { "type": "Min", "field": "price" },
      { "type": "Max", "field": "price" },
      { "type": "Sum", "field": "price" },
      { "type": "Count", "field": "price" }
    ]
  }'
```

| Type | Description |
|------|-------------|
| `Terms` | Bucket documents by field value |
| `Min` | Minimum value |
| `Max` | Maximum value |
| `Sum` | Sum of all values |
| `Average` | Average value |
| `Count` | Number of non-null values |
