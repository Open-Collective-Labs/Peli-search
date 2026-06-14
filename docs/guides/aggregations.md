# Aggregations

Aggregations summarize and group your data. They run alongside search queries and return statistical metrics.

## Terms Aggregation

Group documents by field values and count them:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match_all": {} },
    "aggregations": {
      "by_brand": {
        "terms": {
          "field": "brand",
          "size": 10
        }
      }
    }
  }'
```

Response:

```json
{
  "hits": [...],
  "aggregations": {
    "by_brand": {
      "buckets": [
        { "key": "apple", "count": 42 },
        { "key": "dell", "count": 35 },
        { "key": "lenovo", "count": 28 }
      ]
    }
  }
}
```

## Metrics Aggregation

Compute numeric statistics on a field:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match_all": {} },
    "aggregations": {
      "avg_price": {
        "avg": { "field": "price" }
      },
      "price_stats": {
        "stats": { "field": "price" }
      }
    }
  }'
```

| Metric | Description |
|--------|-------------|
| `min` | Minimum value |
| `max` | Maximum value |
| `sum` | Sum of all values |
| `avg` | Average value |
| `count` | Number of values |
| `stats` | All of the above |

## Histograms

Group numeric values into intervals:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match_all": {} },
    "aggregations": {
      "price_ranges": {
        "histogram": {
          "field": "price",
          "interval": 100
        }
      }
    }
  }'
```

## Nested Aggregations

Combine aggregations for drill-down analysis:

```bash
curl -X POST http://127.0.0.1:8080/indexes/products/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": { "match_all": {} },
    "aggregations": {
      "by_brand": {
        "terms": { "field": "brand" },
        "aggregations": {
          "avg_price": {
            "avg": { "field": "price" }
          }
        }
      }
    }
  }'
```
