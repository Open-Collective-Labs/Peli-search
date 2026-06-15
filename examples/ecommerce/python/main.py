from pelisearch import PeliSearch, SearchRequest

client = PeliSearch("http://localhost:7700")
INDEX = "products"

products = [
    {"id": "p1", "fields": {"title": "Wireless Mouse", "category": "electronics", "price": 29.99}},
    {"id": "p2", "fields": {"title": "Mechanical Keyboard", "category": "electronics", "price": 89.99}},
    {"id": "p3", "fields": {"title": "Running Shoes", "category": "sports", "price": 120.0}},
]

client.create_index(INDEX)
client.bulk_add_documents(INDEX, products)

results = client.search(
    INDEX,
    SearchRequest(q="keyboard", filter="category = electronics", sort=["price:asc"]),
)

print(f"Found {len(results.hits)} products")
for hit in results.hits:
    print(f"- {hit.fields['title']} (${hit.fields['price']})")

client.delete_index(INDEX)
