from __future__ import annotations

from typing import Callable

from .types import IndexCreatedResponse, IndexInfo


class IndexOperations:
    def __init__(self, request: Callable[..., object]) -> None:
        self._request = request

    def list_indexes(self) -> list[str]:
        body = self._request("GET", "/indexes")
        return body["indexes"]

    def get_index(self, name: str) -> IndexInfo:
        body = self._request("GET", f"/indexes/{_esc(name)}")
        return IndexInfo(**body)

    def create_index(self, name: str) -> IndexCreatedResponse:
        body = self._request("POST", "/indexes", {"name": name})
        return IndexCreatedResponse(**body)

    def delete_index(self, name: str) -> None:
        self._request("DELETE", f"/indexes/{_esc(name)}")


def _esc(s: str) -> str:
    import urllib.parse

    return urllib.parse.quote(s, safe="")
