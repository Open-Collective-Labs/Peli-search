import os

import pytest

from pelisearch import (
    MatchQuery,
    PeliSearch,
    PeliSearchError,
    SearchRequest,
    SortField,
)

BASE_URL = os.environ.get("PELISEARCH_TEST_URL", "http://127.0.0.1:7700")
INDEX = "sdk_py_test"


def reset_index(client, name: str) -> None:
    try:
        client.delete_index(name)
    except PeliSearchError:
        pass
    client.create_index(name)


@pytest.fixture(scope="module")
def client():
    c = PeliSearch(BASE_URL)
    c.health()
    reset_index(c, INDEX)
    yield c
    try:
        c.delete_index(INDEX)
    except PeliSearchError:
        pass
    c.close()


def test_index_management(client):
    temp = "sdk_py_index_crud"
    reset_index(client, temp)
    assert temp in client.list_indexes()
    info = client.get_index(temp)
    assert info.name == temp
    assert info.document_count == 0
    client.delete_index(temp)
    assert temp not in client.list_indexes()


def test_missing_index_error(client):
    with pytest.raises(PeliSearchError) as exc:
        client.get_index("nonexistent_sdk_index")
    assert exc.value.status == 404


def test_documents(client):
    reset_index(client, INDEX)
    client.add_document(INDEX, "d1", {"title": "Mouse", "category": "electronics", "price": 29.99})
    doc = client.get_document(INDEX, "d1")
    assert "title" in doc.get("fields", doc)

    bulk = client.bulk_add_documents(
        INDEX,
        [{"id": "d2", "fields": {"title": "Keyboard", "category": "electronics", "price": 89.99}}],
    )
    assert bulk.documents[0].status == "created"
    client.delete_document(INDEX, "d1")


def test_search_legacy_q(client):
    reset_index(client, INDEX)
    client.bulk_add_documents(
        INDEX,
        [
            {"id": "p1", "fields": {"title": "Wireless Mouse", "category": "electronics"}},
            {"id": "p2", "fields": {"title": "Mechanical Keyboard", "category": "electronics"}},
        ],
    )
    results = client.search(INDEX, SearchRequest(q="mouse"))
    assert len(results.hits) > 0
    assert results.total > 0
    for hit in results.hits:
        assert hit.index == INDEX
        assert hit.document_id
        assert hit.score >= 0


def test_search_dsl(client):
    reset_index(client, INDEX)
    client.bulk_add_documents(
        INDEX,
        [{"id": "p2", "fields": {"title": "Mechanical Keyboard", "category": "electronics"}}],
    )
    results = client.search(
        INDEX,
        SearchRequest(
            query=MatchQuery(match={"title": "keyboard"}),
            sort=[SortField(field="title")],
        ),
    )
    assert len(results.hits) > 0
    assert results.total > 0


def test_search_pagination(client):
    reset_index(client, INDEX)
    client.bulk_add_documents(
        INDEX,
        [
            {"id": "a", "fields": {"title": "Alpha"}},
            {"id": "b", "fields": {"title": "Beta"}},
            {"id": "c", "fields": {"title": "Gamma"}},
        ],
    )
    all_results = client.search(INDEX, SearchRequest(q="alpha beta gamma"))
    assert len(all_results.hits) >= 2

    paged = client.search(INDEX, SearchRequest(q="alpha beta gamma", from_=0, size=1))
    assert len(paged.hits) <= 1
