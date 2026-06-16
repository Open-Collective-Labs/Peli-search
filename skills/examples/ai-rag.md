# AI RAG (Retrieval-Augmented Generation)

## Workflow

When integrating PeliSearch with an LLM for question answering, follow this pipeline:

```
User Question
      ↓
PeliSearch Query (retrieve relevant documents)
      ↓
Top-N Documents (hits with content)
      ↓
Inject into LLM Context as source material
      ↓
LLM generates answer grounded in retrieved documents
```

## Implementation

### 1. Search for Relevant Content

```typescript
async function retrieveContext(question: string) {
  const results = await client.search("knowledge-base", {
    query: { match: { content: question } },
    size: 5,
  })
  return results.hits.map(h => ({
    id: h.document_id,
    content: h.highlighted?.content ?? "",
    score: h.score,
  }))
}
```

### 2. Build Prompt

```typescript
function buildPrompt(question: string, documents: Array<{ content: string }>) {
  const context = documents.map((d, i) => `[${i + 1}] ${d.content}`).join("\n\n")
  return `Answer the question based only on the provided context.

Context:
${context}

Question: ${question}

Answer:`
}
```

### 3. Query LLM

Send the prompt to your LLM (OpenAI, Anthropic, local model, etc.).

## Best Practices

- Retrieve 3-5 documents for context
- Use `highlight: true` to surface relevant excerpts
- Include document metadata (title, URL) in the context for citation
- Set `size` to limit results — don't overload the LLM context window
- Always retrieve before generating — never let the LLM answer without context

## Agent Guidance

When asked to implement RAG:

1. Create an index for the knowledge base
2. Index documents with a `content` field for search
3. On each user question, search PeliSearch for relevant documents
4. Format results into an LLM prompt as source context
5. Only then send to the LLM for answer generation
