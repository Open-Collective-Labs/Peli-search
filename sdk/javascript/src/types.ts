export interface SearchHit {
  document_id: string
  score: number
  index?: string
  fields?: Record<string, unknown>
  highlights?: Record<string, string[]>
}

export interface SearchResponse {
  hits: SearchHit[]
  aggregations: Record<string, unknown>
  total_hits?: number
  page?: number
  page_size?: number
  facet_distributions?: Record<string, Record<string, number>>
}

export interface IndexInfo {
  name: string
  document_count: number
  fields: { name: string; type: string }[]
}

export interface IndexCreatedResponse {
  name: string
}

export interface DocumentCreatedResponse {
  id: string
}

export interface BulkDocumentResult {
  id: string
  status: "created" | "error"
  error: string | null
}

export interface BulkResponse {
  documents: BulkDocumentResult[]
}

export interface ErrorResponse {
  error: string
}

export interface MatchQuery {
  match: { [field: string]: string }
}

export interface TermQuery {
  term: { [field: string]: string }
}

export interface RangeCondition {
  gte?: number
  lte?: number
  gt?: number
  lt?: number
}

export interface RangeQuery {
  range: { [field: string]: RangeCondition }
}

export type QueryClause = MatchQuery | TermQuery | RangeQuery

export interface SearchRequest {
  q?: string
  query?: QueryClause
  filter?: string
  sort?: string[]
  page?: number
  page_size?: number
  facets?: string[]
  highlight?: boolean
  highlight_fields?: string[]
  highlight_pre_tag?: string
  highlight_post_tag?: string
}

export interface ClientOptions {
  /** Base URL, e.g. `http://localhost:7700`. Defaults to `http://localhost:7700`. */
  host?: string
}

export type RequestFn = <T>(method: string, path: string, body?: unknown) => Promise<T>
