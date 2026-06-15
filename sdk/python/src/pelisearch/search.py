from __future__ import annotations

from dataclasses import asdict
from typing import Callable

from .types import SearchHit, SearchRequest, SearchResponse


class SearchOperations:
    def __init__(self, request: Callable[..., object]) -> None:
        self._request = request

    def search(self, index: str, request: SearchRequest) -> SearchResponse:
        payload = _drop_none(asdict(request))
        body = self._request(
            "POST",
            f"/indexes/{_esc(index)}/search",
            payload,
        )
        hits = [
            SearchHit(
                document_id=h["document_id"],
                score=h["score"],
                index=h.get("index"),
                fields=h.get("fields"),
                highlights=h.get("highlights"),
            )
            for h in body["hits"]
        ]
        return SearchResponse(
            hits=hits,
            aggregations=body.get("aggregations") or {},
            total_hits=body.get("total_hits"),
            page=body.get("page"),
            page_size=body.get("page_size"),
            facet_distributions=body.get("facet_distributions"),
        )


def _esc(s: str) -> str:
    import urllib.parse

    return urllib.parse.quote(s, safe="")


def _drop_none(d: dict) -> dict:
    return {k: v for k, v in d.items() if v is not None}
