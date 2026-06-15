import type { RequestFn, IndexInfo, IndexCreatedResponse } from "./types.js"

export class IndexesModule {
  constructor(private readonly request: RequestFn) {}

  async list(): Promise<string[]> {
    const body = await this.request<{ indexes: string[] }>("GET", "/indexes")
    return body.indexes
  }

  async get(name: string): Promise<IndexInfo> {
    return this.request<IndexInfo>("GET", `/indexes/${encodeURIComponent(name)}`)
  }

  async create(name: string): Promise<IndexCreatedResponse> {
    return this.request<IndexCreatedResponse>("POST", "/indexes", { name })
  }

  async delete(name: string): Promise<void> {
    await this.request<void>("DELETE", `/indexes/${encodeURIComponent(name)}`)
  }
}
