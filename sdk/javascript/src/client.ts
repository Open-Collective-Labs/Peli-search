import { DocumentsModule } from "./documents.js"
import { PeliSearchError } from "./errors.js"
import { IndexesModule } from "./indexes.js"
import { SearchModule } from "./search.js"
import type { ClientOptions, RequestFn, ErrorResponse, SearchRequest, SearchResponse } from "./types.js"

const DEFAULT_HOST = "http://localhost:7700"

export class PeliSearchClient {
  private readonly request: RequestFn
  private readonly searchModule: SearchModule

  readonly indexes: IndexesModule
  readonly documents: DocumentsModule

  constructor(opts: ClientOptions = {}) {
    const baseUrl = (opts.host ?? DEFAULT_HOST).replace(/\/+$/, "")
    const headers = { "Content-Type": "application/json" }
    this.request = <T>(method: string, path: string, body?: unknown) =>
      PeliSearchClient.doFetch<T>(baseUrl, headers, method, path, body)
    this.indexes = new IndexesModule(this.request)
    this.documents = new DocumentsModule(this.request)
    this.searchModule = new SearchModule(this.request)
  }

  // ── Indexes ──────────────────────────────────────────────────

  async createIndex(name: string) {
    return this.indexes.create(name)
  }

  async deleteIndex(name: string) {
    await this.indexes.delete(name)
  }

  async getIndex(name: string) {
    return this.indexes.get(name)
  }

  async listIndexes() {
    return this.indexes.list()
  }

  // ── Documents ────────────────────────────────────────────────

  async addDocument(index: string, id: string, fields: Record<string, unknown>) {
    return this.documents.add(index, id, fields)
  }

  async getDocument(index: string, id: string) {
    return this.documents.get(index, id)
  }

  async deleteDocument(index: string, id: string) {
    await this.documents.delete(index, id)
  }

  async bulkAddDocuments(
    index: string,
    documents: { id: string; fields: Record<string, unknown> }[],
  ) {
    return this.documents.bulkAdd(index, documents)
  }

  // ── Search ───────────────────────────────────────────────────

  async search(index: string, query: SearchRequest): Promise<SearchResponse> {
    return this.searchModule.search(index, query)
  }

  // ── Health ───────────────────────────────────────────────────

  async health(): Promise<void> {
    await this.request<void>("GET", "/health")
  }

  async ready(): Promise<void> {
    await this.request<void>("GET", "/ready")
  }

  // ── Internal ─────────────────────────────────────────────────

  private static async doFetch<T>(
    baseUrl: string,
    headers: Record<string, string>,
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${baseUrl}${path}`
    const opts: RequestInit = {
      method,
      headers: { ...headers },
    }
    if (body !== undefined) {
      opts.body = JSON.stringify(body)
    }

    const res = await fetch(url, opts)
    const text = await res.text()

    if (!res.ok) {
      let msg: string
      let parsed: unknown
      try {
        parsed = JSON.parse(text)
        msg = (parsed as ErrorResponse).error ?? res.statusText
      } catch {
        msg = res.statusText
      }
      throw new PeliSearchError(msg, res.status, parsed)
    }

    if (text.length === 0) return undefined as T
    return JSON.parse(text) as T
  }
}
