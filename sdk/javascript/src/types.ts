export interface SearchHit {
  document_id: string
  score: number
  index: string
  highlighted?: Record<string, string>
}

export interface SearchResponse {
  hits: SearchHit[]
  total: number
  aggregations: Record<string, unknown>
}

export interface IndexInfo {
  name: string
  document_count: number
  fields: { name: string; field_type: string; required: boolean }[]
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

export interface CoreQuery {
  type: "Match" | "Term" | "Range" | "Bool" | "Phrase" | "Fuzzy" | "Prefix" | "MultiMatch" | "ConstantScore" | "DisMax" | "MatchAll" | "MatchNone"
  field?: string
  value?: unknown
  gte?: number
  gt?: number
  lte?: number
  lt?: number
  must?: CoreQuery[]
  filter?: CoreQuery[]
  must_not?: CoreQuery[]
  should?: CoreQuery[]
  slop?: number
  max_edit_distance?: number
  prefix_length?: number
}

export type QueryClause = MatchQuery | TermQuery | RangeQuery | CoreQuery

export interface SortField {
  field: string
  order: "Asc" | "Desc"
}

export interface SearchRequest {
  q?: string
  query?: QueryClause
  filters?: QueryClause[]
  sort?: SortField[]
  from?: number
  size?: number
  highlight?: boolean
  aggregations?: unknown[]
}

export interface ClientOptions {
  /** Base URL, e.g. `http://localhost:7700`. Defaults to `http://localhost:7700`. */
  host?: string
  /** API key for authenticated endpoints. Sent as `X-Api-Key` header. */
  apiKey?: string
}

export type RequestFn = <T>(method: string, path: string, body?: unknown) => Promise<T>
