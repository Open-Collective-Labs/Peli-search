from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class SearchHit:
    document_id: str
    score: float
    index: Optional[str] = None
    fields: Optional[dict[str, Any]] = None
    highlights: Optional[dict[str, list[str]]] = None


@dataclass
class SearchResponse:
    hits: list[SearchHit]
    aggregations: dict[str, Any] = field(default_factory=dict)
    total_hits: Optional[int] = None
    page: Optional[int] = None
    page_size: Optional[int] = None
    facet_distributions: Optional[dict[str, dict[str, int]]] = None


@dataclass
class IndexInfo:
    name: str
    document_count: int
    fields: list[dict[str, str]]


@dataclass
class IndexCreatedResponse:
    name: str


@dataclass
class DocumentCreatedResponse:
    id: str


@dataclass
class BulkDocumentResult:
    id: str
    status: str
    error: Optional[str]


@dataclass
class BulkResponse:
    documents: list[BulkDocumentResult]


@dataclass
class MatchQuery:
    match: dict[str, str]


@dataclass
class TermQuery:
    term: dict[str, str]


@dataclass
class RangeCondition:
    gte: Optional[float] = None
    lte: Optional[float] = None
    gt: Optional[float] = None
    lt: Optional[float] = None


@dataclass
class RangeQuery:
    range: dict[str, RangeCondition]


QueryClause = MatchQuery | TermQuery | RangeQuery


@dataclass
class SearchRequest:
    q: Optional[str] = None
    query: Optional[QueryClause] = None
    filter: Optional[str] = None
    sort: Optional[list[str]] = None
    page: Optional[int] = None
    page_size: Optional[int] = None
    facets: Optional[list[str]] = None
    highlight: Optional[bool] = None
    highlight_fields: Optional[list[str]] = None
    highlight_pre_tag: Optional[str] = None
    highlight_post_tag: Optional[str] = None
