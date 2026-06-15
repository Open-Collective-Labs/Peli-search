import type { RequestFn, SearchRequest, SearchResponse } from "./types.js"

export class SearchModule {
  constructor(private readonly request: RequestFn) {}

  async search(index: string, request: SearchRequest): Promise<SearchResponse> {
    return this.request<SearchResponse>(
      "POST",
      `/indexes/${encodeURIComponent(index)}/search`,
      request,
    )
  }
}
