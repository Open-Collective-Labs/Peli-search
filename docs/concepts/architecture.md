# Architecture

## High-Level Architecture

Peli Search follows a layered design with four core components connected by a data pipeline.

```
┌──────────────────────────────────────────────┐
│                  API Layer                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │  REST    │  │ Embedded │  │   SDK    │    │
│  │  Server  │  │   API    │  │  Client  │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
├───────┴──────────────┴──────────────┴────────┤
│              Query Engine                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Parser   │  │ Planner  │  │ Executor │    │
│  └──────────┘  └──────────┘  └──────────┘    │
├──────────────────────────────────────────────┤
│              Index Engine                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Mappings │  │ Analyzer │  │ Inverted │    │
│  │          │  │          │  │  Index   │    │
│  └──────────┘  └──────────┘  └──────────┘    │
├──────────────────────────────────────────────┤
│              Storage Engine                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │   WAL    │  │ Segments │  │  Merger  │    │
│  └──────────┘  └──────────┘  └──────────┘    │
└──────────────────────────────────────────────┘
```

## Core Components

### API Layer
Exposes search functionality via REST and embedded APIs. Handles authentication, request validation, and response formatting.

### Query Engine
Parses search queries, builds an execution plan, and coordinates retrieval across segments. Supports match, phrase, boolean, term, and range queries.

### Index Engine
Manages mappings, analyzes text, and maintains inverted indexes. Defines how documents are tokenized and stored.

### Storage Engine
Persists data through a write-ahead log (WAL), maintains immutable segments, and runs background merges for query performance.

## Data Flow

1. **Write path**: Document → API → Mappings → Analyzer → Inverted Index → WAL → Segment
2. **Read path**: Query → API → Query Engine → Segment Scan → Scoring → Results
3. **Merge path**: Segments → Merger → New Compacted Segment → Old Segments Removed
