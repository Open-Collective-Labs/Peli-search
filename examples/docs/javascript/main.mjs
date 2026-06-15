import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient()
const INDEX = "docs"

const pages = [
  { id: "install", fields: { title: "Installation", section: "getting-started", content: "Install PeliSearch from source or binary." } },
  { id: "quickstart", fields: { title: "Quick Start", section: "getting-started", content: "Create an index and run your first search." } },
  { id: "filtering", fields: { title: "Filtering", section: "guides", content: "Filter results with field expressions." } },
]

await client.createIndex(INDEX)
await client.bulkAddDocuments(INDEX, pages)

const results = await client.search(INDEX, {
  query: { match: { content: "index" } },
  filter: "section = getting-started",
})

console.log("Getting started pages mentioning 'index':")
for (const hit of results.hits) {
  console.log(`- ${hit.fields.title}`)
}

await client.deleteIndex(INDEX)
