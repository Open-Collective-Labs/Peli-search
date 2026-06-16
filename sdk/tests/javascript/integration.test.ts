import { beforeAll, describe, expect, it } from "vitest"
import { PeliSearchClient, PeliSearchError } from "@pelisearch/client"

const baseUrl = process.env.PELISEARCH_TEST_URL ?? "http://127.0.0.1:7700"
const client = new PeliSearchClient({ host: baseUrl })
const INDEX = "sdk_js_test"

async function resetIndex(name: string) {
  try {
    await client.deleteIndex(name)
  } catch {
    // index may not exist
  }
  await client.createIndex(name)
}

beforeAll(async () => {
  await client.health()
  await resetIndex(INDEX)
})

describe("index management", () => {
  it("creates, lists, gets, and deletes indexes", async () => {
    const temp = "sdk_js_index_crud"
    await resetIndex(temp)

    const names = await client.listIndexes()
    expect(names).toContain(temp)

    const info = await client.getIndex(temp)
    expect(info.name).toBe(temp)
    expect(info.document_count).toBe(0)

    await client.deleteIndex(temp)
    const after = await client.listIndexes()
    expect(after).not.toContain(temp)
  })

  it("returns typed errors for missing indexes", async () => {
    try {
      await client.getIndex("nonexistent_sdk_index")
      expect.fail("expected error")
    } catch (err) {
      expect(err).toBeInstanceOf(PeliSearchError)
      expect((err as PeliSearchError).status).toBe(404)
    }
  })
})

describe("documents", () => {
  beforeAll(async () => {
    await resetIndex(INDEX)
  })

  it("adds, gets, bulk adds, and deletes documents", async () => {
    await client.addDocument(INDEX, "d1", { title: "Mouse", category: "electronics", price: 29.99 })
    const doc = await client.getDocument(INDEX, "d1")
    expect(doc.fields?.title ?? doc.title).toBeTruthy()

    const bulk = await client.bulkAddDocuments(INDEX, [
      { id: "d2", fields: { title: "Keyboard", category: "electronics", price: 89.99 } },
    ])
    expect(bulk.documents[0].status).toBe("created")

    await client.deleteDocument(INDEX, "d1")
  })
})

describe("search", () => {
  beforeAll(async () => {
    await resetIndex(INDEX)
    await client.bulkAddDocuments(INDEX, [
      { id: "p1", fields: { title: "Wireless Mouse", category: "electronics", price: 29.99 } },
      { id: "p2", fields: { title: "Mechanical Keyboard", category: "electronics", price: 89.99 } },
    ])
  })

  it("searches with legacy q", async () => {
    const results = await client.search(INDEX, { q: "mouse" })
    expect(results.hits.length).toBeGreaterThan(0)
    expect(results.total).toBeGreaterThan(0)
    for (const hit of results.hits) {
      expect(hit.index).toBe(INDEX)
      expect(hit.document_id).toBeTruthy()
      expect(hit.score).toBeGreaterThanOrEqual(0)
    }
  })

  it("searches with DSL match", async () => {
    const results = await client.search(INDEX, {
      query: { match: { title: "keyboard" } },
    })
    expect(results.hits.length).toBeGreaterThan(0)
  })

  it("supports pagination", async () => {
    const results = await client.search(INDEX, { q: "mouse keyboard", from: 0, size: 1 })
    expect(results.hits.length).toBeLessThanOrEqual(1)
  })
})
