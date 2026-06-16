/**
 * Shared types used across all PeliSearch SDKs.
 *
 * See docs/openapi.yaml for the complete API specification.
 *
 * Core operations:
 * - Index CRUD (create, get, list, delete)
 * - Document CRUD (add, get, delete, bulk add)
 * - Search (query DSL, filters, sorting, aggregations, highlighting)
 */

export interface IndexInfo {
  name: string
  document_count: number
  fields: { name: string; type: string }[]
}

export interface SearchHit {
  index: string
  document_id: string
  score: number
  highlighted?: Record<string, string>
}

export interface SearchResponse {
  hits: SearchHit[]
  total: number
  aggregations: Record<string, unknown>
}

export interface RangeCondition {
  gte?: number
  lte?: number
  gt?: number
  lt?: number
}

export type QueryClause =
  | { match: Record<string, string> }
  | { term: Record<string, string> }
  | { range: Record<string, RangeCondition> }

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
