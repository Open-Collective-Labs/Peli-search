# PeliSearch

> Lightweight, embeddable search engine built for modern applications.
>
> Full-text search, filtering, aggregations, and vector search in a single developer-friendly engine.

---

## Why PeliSearch?

Modern search engines are powerful, but often come with significant operational complexity.

PeliSearch is designed around a simple idea:

**Search should be easy to embed, easy to deploy, and easy to understand.**

Whether you're building a SaaS application, an internal tool, a documentation site, or a local desktop application, PeliSearch provides fast search capabilities without requiring a large distributed cluster.

### Core Principles

* Embeddable-first architecture
* Single-node simplicity
* High-performance search
* Modern developer experience
* Open-source and Apache 2.0 licensed
* Built for lexical and semantic search

---

## Features

### Full-Text Search

Fast and relevant search powered by BM25 ranking.

```json
{
  "query": {
    "match": {
      "title": "electric bike"
    }
  }
}
```

### Filtering

Filter results using exact matches, ranges, and dates.

```json
{
  "filter": {
    "range": {
      "price": {
        "lte": 1500
      }
    }
  }
}
```

### Aggregations

Generate analytics and faceted search experiences.

* Terms aggregations
* Count
* Min
* Max
* Average
* Sum

### Vector Search (Roadmap)

Semantic search powered by embeddings and vector similarity.

### Hybrid Search (Roadmap)

Combine lexical and vector search using modern ranking strategies.

---

## Quick Example

### Create an Index

```javascript
const engine = new PeliSearch("./data");

await engine.createIndex("products");
```

### Index Documents

```javascript
await engine.index("products", {
  id: 1,
  title: "Electric Bike",
  price: 999
});
```

### Search

```javascript
const results = await engine.search("products", {
  query: {
    match: {
      title: "bike"
    }
  }
});
```

---

## Architecture

```text
Applications
      │
      ▼
 SDK Layer
      │
      ▼
 Search Engine
      │
 ┌────┼────┐
 ▼    ▼    ▼
Text Filter Ranking
      │
      ▼
 Storage Engine
      │
      ▼
 WAL + Segments
```

---

## Supported SDKs

Planned SDKs:

* Rust
* Node.js
* Python
* Go

All SDKs expose a consistent API.

---

## Documentation

### Getting Started

* Installation
* Quick Start
* First Index
* First Search

### Concepts

* Architecture
* Documents
* Indexes
* Mappings
* Storage Engine
* Search Ranking

### Guides

* Full-Text Search
* Filtering
* Sorting
* Pagination
* Aggregations
* Vector Search
* Hybrid Search

### Architecture

* Storage Engine
* Inverted Index
* Query Engine
* Ranking Engine
* Vector Engine

See the full documentation in the `/docs` directory.

---

## Roadmap

### Version 1

* Single-node architecture
* Full-text search
* Filtering
* Sorting
* Aggregations
* Embedded SDKs

### Version 2

* Vector search
* Hybrid search
* Advanced ranking

### Future

* Replication
* Clustering
* Distributed search

---

## Status

PeliSearch is currently under active development.

The initial release focuses on building a fast, embeddable search engine with a strong foundation before expanding into vector and distributed search capabilities.

---

## License

Apache License 2.0

Copyright (c) PeliSearch Contributors
