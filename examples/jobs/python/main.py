from pelisearch import PeliSearch, SearchRequest

client = PeliSearch("http://localhost:7700")
INDEX = "jobs"

jobs = [
    {"id": "j1", "fields": {"title": "Backend Engineer", "company": "Acme", "location": "remote", "skills": "rust go"}},
    {"id": "j2", "fields": {"title": "Frontend Developer", "company": "Beta", "location": "nyc", "skills": "typescript react"}},
    {"id": "j3", "fields": {"title": "Search Engineer", "company": "Acme", "location": "remote", "skills": "rust search"}},
]

client.create_index(INDEX)
client.bulk_add_documents(INDEX, jobs)

results = client.search(
    INDEX,
    SearchRequest(
        q="engineer",
        filter="location = remote",
        facets=["company"],
        page=1,
        page_size=10,
    ),
)

print(f"Page {results.page if results.page else 1}: {len(results.hits)} remote engineer roles")
for hit in results.hits:
    print(f"- {hit.fields['title']} at {hit.fields['company']}")
if results.facet_distributions:
    print("Companies:", results.facet_distributions.get("company"))

client.delete_index(INDEX)
