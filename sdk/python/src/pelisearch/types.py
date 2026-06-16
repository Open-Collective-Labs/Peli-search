from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class SearchHit:
    index: str
    document_id: str
    score: float
    highlighted: Optional[dict[str, str]] = None


@dataclass
class SearchResponse:
    hits: list[SearchHit]
    aggregations: dict[str, Any] = field(default_factory=dict)
    total: int = 0


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
class SortField:
    field: str
    order: str = "Asc"


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
    filters: Optional[list[QueryClause]] = None
    sort: Optional[list[SortField]] = None
    from_: Optional[int] = None
    size: Optional[int] = None
    highlight: Optional[bool] = None
    aggregations: Optional[list[Any]] = None
