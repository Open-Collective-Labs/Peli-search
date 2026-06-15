import { PeliSearchClient } from "@pelisearch/client"

const client = new PeliSearchClient()
const INDEX = "jobs"

const jobs = [
  { id: "j1", fields: { title: "Backend Engineer", company: "Acme", location: "remote", skills: "rust go" } },
  { id: "j2", fields: { title: "Frontend Developer", company: "Beta", location: "nyc", skills: "typescript react" } },
  { id: "j3", fields: { title: "Search Engineer", company: "Acme", location: "remote", skills: "rust search" } },
]

await client.createIndex(INDEX)
await client.bulkAddDocuments(INDEX, jobs)

const results = await client.search(INDEX, {
  q: "engineer",
  filter: "location = remote",
  facets: ["company"],
  page: 1,
  page_size: 10,
})

console.log(`Page ${results.page ?? 1}: ${results.hits.length} remote engineer roles`)
for (const hit of results.hits) {
  console.log(`- ${hit.fields.title} at ${hit.fields.company}`)
}
if (results.facet_distributions) {
  console.log("Companies:", results.facet_distributions.company)
}

await client.deleteIndex(INDEX)
