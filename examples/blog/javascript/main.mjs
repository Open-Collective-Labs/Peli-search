import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient()
const INDEX = "posts"

const posts = [
  { id: "1", fields: { title: "Getting Started with PeliSearch", body: "Learn how to index and search documents." } },
  { id: "2", fields: { title: "Advanced Filtering", body: "Use filters to narrow search results." } },
  { id: "3", fields: { title: "Building a Blog", body: "Add search to your content site." } },
]

await client.createIndex(INDEX)
await client.bulkAddDocuments(INDEX, posts)

const results = await client.search(INDEX, {
  q: "filter",
  highlight: true,
  highlight_fields: ["title", "body"],
})

for (const hit of results.hits) {
  console.log(hit.document_id, hit.fields.title)
  if (hit.highlights) console.log("  highlights:", hit.highlights)
}

await client.deleteIndex(INDEX)
