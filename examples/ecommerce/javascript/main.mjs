import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient()

const INDEX = "products"

const products = [
  { id: "p1", fields: { title: "Wireless Mouse", category: "electronics", price: 29.99 } },
  { id: "p2", fields: { title: "Mechanical Keyboard", category: "electronics", price: 89.99 } },
  { id: "p3", fields: { title: "Running Shoes", category: "sports", price: 120.0 } },
]

await client.createIndex(INDEX)
await client.bulkAddDocuments(INDEX, products)

const results = await client.search(INDEX, {
  q: "keyboard",
  filter: "category = electronics",
  sort: ["price:asc"],
})

console.log(`Found ${results.hits.length} products`)
for (const hit of results.hits) {
  console.log(`- ${hit.fields.title} ($${hit.fields.price})`)
}

await client.deleteIndex(INDEX)
