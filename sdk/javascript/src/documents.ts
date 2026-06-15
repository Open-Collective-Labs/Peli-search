import type { RequestFn, DocumentCreatedResponse, BulkResponse } from "./types.js"

export interface AddDocumentPayload {
  id: string
  fields: Record<string, unknown>
}

export class DocumentsModule {
  constructor(private readonly request: RequestFn) {}

  async add(
    index: string,
    id: string,
    fields: Record<string, unknown>,
  ): Promise<DocumentCreatedResponse> {
    return this.request<DocumentCreatedResponse>(
      "POST",
      `/indexes/${encodeURIComponent(index)}/documents`,
      { id, fields },
    )
  }

  async get(index: string, id: string): Promise<Record<string, unknown>> {
    return this.request<Record<string, unknown>>(
      "GET",
      `/indexes/${encodeURIComponent(index)}/documents/${encodeURIComponent(id)}`,
    )
  }

  async delete(index: string, id: string): Promise<void> {
    await this.request<void>(
      "DELETE",
      `/indexes/${encodeURIComponent(index)}/documents/${encodeURIComponent(id)}`,
    )
  }

  async bulkAdd(
    index: string,
    documents: AddDocumentPayload[],
  ): Promise<BulkResponse> {
    return this.request<BulkResponse>(
      "POST",
      `/indexes/${encodeURIComponent(index)}/documents/bulk`,
      { documents },
    )
  }
}
