from __future__ import annotations

from dataclasses import asdict
from typing import Callable

from .types import SearchHit, SearchRequest, SearchResponse


class SearchOperations:
    def __init__(self, request: Callable[..., object]) -> None:
        self._request = request

    def search(self, index: str, request: SearchRequest) -> SearchResponse:
        payload = _serialize_request(request)
        body = self._request(
            "POST",
            f"/indexes/{_esc(index)}/search",
            payload,
        )
        hits = [
            SearchHit(
                document_id=h["document_id"],
                score=h["score"],
                index=h["index"],
                highlighted=h.get("highlighted"),
            )
            for h in body["hits"]
        ]
        return SearchResponse(
            hits=hits,
            aggregations=body.get("aggregations", {}),
            total=body.get("total", 0),
        )


def _serialize(d: dict) -> dict:
    """Recursively convert dataclass dicts to plain dicts for JSON serialization."""
    if isinstance(d, dict):
        return {k: _serialize(v) for k, v in d.items()}
    elif isinstance(d, list):
        return [_serialize(v) for v in d]
    return d


def _esc(s: str) -> str:
    import urllib.parse

    return urllib.parse.quote(s, safe="")


def _drop_none(d: dict) -> dict:
    return {k: v for k, v in d.items() if v is not None}


def _serialize_request(req: SearchRequest) -> dict:
    d = asdict(req)
    if "from_" in d:
        d["from"] = d.pop("from_")
    return _drop_none(_serialize(d))
