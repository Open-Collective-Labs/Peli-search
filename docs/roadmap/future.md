# Future

Potential features under consideration for releases beyond v2.

## Replication

- Primary-replica replication model
- Async replication with configurable consistency
- Automatic failover and leader election
- Read replicas for horizontal scaling

## Clustering

- Distributed index across multiple nodes
- Consistent hashing for document routing
- Gossip protocol for cluster membership
- Node discovery and health checking

## Distributed Search

- Scatter-gather query execution
- Distributed top-k result merging
- Cross-node aggregation
- Query fan-out with per-node timeouts

## Query Planner

- Cost-based query optimization
- Segment pruning (skip irrelevant segments)
- Filter pushdown optimization
- Early termination for top-k queries
- Precomputed aggregations

## Additional Features

| Feature | Description |
|---------|-------------|
| Synonyms | Configurable synonym sets for query expansion |
| Spell correction | Did-you-mean suggestions |
| Highlighting | Snippet generation with matched terms |
| Faceted search | Dynamic facet counts in search results |
| Geo search | Bounding box and distance queries |
| Custom scoring | Script-based or plugin scoring functions |
