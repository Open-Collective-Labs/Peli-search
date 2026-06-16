# JavaScript Indexing Examples

## Create Index

```typescript
await client.createIndex("articles")
// Returns: { name: "articles" }
```

## Add Single Document

```typescript
await client.addDocument("articles", "doc-1", {
  title: "Getting Started with Rust",
  content: "Rust is a systems programming language...",
  tags: ["programming", "rust"],
  published: "2025-01-15"
})
```

## Bulk Add Documents

```typescript
await client.bulkAddDocuments("articles", [
  { id: "1", fields: { title: "Rust Ownership", content: "..." } },
  { id: "2", fields: { title: "Rust Borrowing", content: "..." } },
])
// Returns: { documents: [{ id: "1", status: "created", error: null }, ...] }
```

## Get / Delete Document

```typescript
const doc = await client.getDocument("articles", "doc-1")
console.log(doc.title)

await client.deleteDocument("articles", "doc-1")
```
