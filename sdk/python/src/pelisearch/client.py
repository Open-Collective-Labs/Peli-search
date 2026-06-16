from __future__ import annotations

import json
from typing import Any, Optional, Union
from urllib.parse import urlparse

import httpx

from .documents import DocumentOperations
from .exceptions import PeliSearchError
from .indexes import IndexOperations
from .search import SearchOperations
from .types import (
    BulkResponse,
    DocumentCreatedResponse,
    IndexCreatedResponse,
    IndexInfo,
    SearchRequest,
    SearchResponse,
)

DEFAULT_HOST = "http://localhost:7700"


class PeliSearchClient:
    """Official Python client for PeliSearch."""

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 7700,
        *,
        base_url: Optional[str] = None,
        api_key: Optional[str] = None,
    ) -> None:
        if base_url is not None:
            self.base_url = base_url.rstrip("/")
        elif host.startswith("http://") or host.startswith("https://"):
            self.base_url = host.rstrip("/")
        else:
            self.base_url = f"http://{host}:{port}"

        self.headers: dict[str, str] = {"Content-Type": "application/json"}
        if api_key:
            self.headers["X-Api-Key"] = api_key
        self._client = httpx.Client(headers=self.headers)

        self.indexes = IndexOperations(self._request)
        self.documents = DocumentOperations(self._request)
        self.search_ops = SearchOperations(self._request)

    @classmethod
    def from_url(cls, url: str, *, api_key: Optional[str] = None) -> PeliSearchClient:
        parsed = urlparse(url)
        if not parsed.scheme or not parsed.netloc:
            raise ValueError(f"invalid PeliSearch URL: {url}")
        return cls(base_url=url, api_key=api_key)

    def __enter__(self) -> PeliSearchClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    def close(self) -> None:
        self._client.close()

    # ── Health ───────────────────────────────────────────────────

    def health(self) -> None:
        self._request("GET", "/health")

    def ready(self) -> None:
        self._request("GET", "/ready")

    def metrics(self) -> dict:
        return self._request("GET", "/metrics")

    # ── Indexes ──────────────────────────────────────────────────

    def list_indexes(self) -> list[str]:
        return self.indexes.list_indexes()

    def get_index(self, name: str) -> IndexInfo:
        return self.indexes.get_index(name)

    def create_index(self, name: str) -> IndexCreatedResponse:
        return self.indexes.create_index(name)

    def delete_index(self, name: str) -> None:
        self.indexes.delete_index(name)

    # ── Documents ────────────────────────────────────────────────

    def add_document(
        self, index: str, id: str, fields: dict[str, Any]
    ) -> DocumentCreatedResponse:
        return self.documents.add_document(index, id, fields)

    def get_document(self, index: str, id: str) -> dict[str, Any]:
        return self.documents.get_document(index, id)

    def delete_document(self, index: str, id: str) -> None:
        self.documents.delete_document(index, id)

    def bulk_add_documents(
        self,
        index: str,
        documents: list[dict[str, Any]],
    ) -> BulkResponse:
        return self.documents.bulk_add_documents(index, documents)

    # ── Search ───────────────────────────────────────────────────

    def search(self, index: str, request: SearchRequest) -> SearchResponse:
        return self.search_ops.search(index, request)

    # ── Internal ─────────────────────────────────────────────────

    def _request(self, method: str, path: str, body: Any = None) -> Any:
        url = f"{self.base_url}{path}"
        data = json.dumps(body) if body is not None else None
        resp = self._client.request(method, url, content=data)

        if not resp.is_success:
            msg = resp.text
            try:
                msg = resp.json().get("error", resp.text)
            except Exception:
                pass
            raise PeliSearchError(msg, resp.status_code)

        text = resp.text
        if not text:
            return None
        return resp.json()


def PeliSearch(
    url: Union[str, None] = None,
    *,
    host: str = "127.0.0.1",
    port: int = 7700,
    api_key: Optional[str] = None,
) -> PeliSearchClient:
    """Create a PeliSearch client. Pass a full URL or host/port."""
    if url is not None:
        return PeliSearchClient.from_url(url, api_key=api_key)
    if host.startswith("http://") or host.startswith("https://"):
        return PeliSearchClient(base_url=host, api_key=api_key)
    return PeliSearchClient(host=host, port=port, api_key=api_key)
