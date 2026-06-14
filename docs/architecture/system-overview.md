# System Overview

## High-Level Design

Peli Search is a lightweight, embeddable search engine built in Rust. The system is organized into four layers that communicate through well-defined interfaces.

```
┌──────────────────────────────────────────────────┐
│                    API Layer                       │
│  ┌─────────────┐  ┌─────────────┐                 │
│  │  REST HTTP   │  │  Embedded   │                 │
│  │   Server     │  │    API      │                 │
│  └──────┬───────┘  └──────┬──────┘                 │
├─────────┴─────────────────┴────────────────────────┤
│                  Query Engine                       │
│  ┌──────────┐  ┌──────────┐  ┌────────────────┐   │
│  │  Parser  │  │  Planner │  │   Executor     │   │
│  │          │  │          │  │  (Multi-Seg.)  │   │
│  └──────────┘  └──────────┘  └────────┬───────┘   │
├────────────────────────────────────────┴───────────┤
│                  Index Engine                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │
│  │ Mappings │  │ Analyzer │  │  Inverted Index  │ │
│  │ Manager  │  │  Chain   │  │  + DocValues     │ │
│  └──────────┘  └──────────┘  └──────────────────┘ │
├────────────────────────────────────────────────────┤
│                 Storage Engine                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │
│  │   WAL    │  │ Segments │  │    Background    │ │
│  │          │  │ (mmap)   │  │     Merger       │ │
│  └──────────┘  └──────────┘  └──────────────────┘ │
└────────────────────────────────────────────────────┘
```

## Component Responsibilities

### API Layer
- HTTP server (Actix-web) for REST API
- Embedded `Index` struct for library use
- Request validation, error mapping, response serialization

### Query Engine
- Parses JSON query DSL into an internal query tree
- Plans execution order (filter pushdown, segment pruning)
- Executes queries across all active segments in parallel
- Collects, scores, sorts, and paginates results

### Index Engine
- Manages field mappings and type definitions
- Runs text analysis (tokenization, stemming, stop-word removal)
- Maintains inverted indexes for text fields
- Maintains DocValues for sorting, filtering, aggregations

### Storage Engine
- WAL for durability and crash recovery
- Immutable segments on disk, memory-mapped for fast access
- Background merger for segment compaction and optimization

## Data Flow

### Write Path
```
Client → API → Index Engine → WAL → Memory Buffer → Segment Flush
```

### Read Path
```
Client → API → Query Engine → Segment Scans → Scoring → Sort → Paginate → Response
```

### Merge Path
```
Segment A ─┐
Segment B ─┤──→ Merger ──→ New Segment ──→ Remove Old Segments
Segment C ─┘
```
