from __future__ import annotations

from typing import Any, Callable

from .types import BulkDocumentResult, BulkResponse, DocumentCreatedResponse


class DocumentOperations:
    def __init__(self, request: Callable[..., object]) -> None:
        self._request = request

    def add_document(
        self, index: str, id: str, fields: dict[str, Any]
    ) -> DocumentCreatedResponse:
        body = self._request(
            "POST",
            f"/indexes/{_esc(index)}/documents",
            {"id": id, "fields": fields},
        )
        return DocumentCreatedResponse(**body)

    def get_document(self, index: str, id: str) -> dict[str, Any]:
        return self._request(
            "GET",
            f"/indexes/{_esc(index)}/documents/{_esc(id)}",
        )

    def delete_document(self, index: str, id: str) -> None:
        self._request(
            "DELETE",
            f"/indexes/{_esc(index)}/documents/{_esc(id)}",
        )

    def bulk_add_documents(
        self,
        index: str,
        documents: list[dict[str, Any]],
    ) -> BulkResponse:
        body = self._request(
            "POST",
            f"/indexes/{_esc(index)}/documents/bulk",
            {"documents": documents},
        )
        docs = [BulkDocumentResult(**d) for d in body["documents"]]
        return BulkResponse(documents=docs)


def _esc(s: str) -> str:
    import urllib.parse

    return urllib.parse.quote(s, safe="")
