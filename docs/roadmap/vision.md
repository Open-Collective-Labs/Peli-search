# Vision

## Long-Term Mission

Make high-quality search infrastructure accessible to every developer, regardless of team size or budget.

## Goals

### 1. Zero-Configuration Operability
A single binary should work out of the box. No external dependencies (no Java, no separate database, no cloud service).

### 2. Embeddable by Design
First-class embedded library support. Developers should be able to add search to any Rust application with one dependency.

### 3. Search Quality
Deliver relevant results from day one. BM25 with sensible defaults, fast indexing, and sub-50ms query latency for indexes up to 10M documents.

### 4. Progressive Complexity
Start with a simple `Index::create()` and `index.search().execute()`. Graduate to custom analyzers, advanced queries, and vector search as your needs grow.

### 5. Open Ecosystem
Apache 2.0 license. Clean, documented APIs. SDKs for Rust, Node.js, Python, and Go.
