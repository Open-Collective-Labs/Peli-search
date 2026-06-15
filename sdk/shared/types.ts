/**
 * Shared types used across all PeliSearch SDKs.
 *
 * See docs/openapi.yaml for the complete API specification.
 *
 * Core operations:
 * - Index CRUD (create, get, list, delete)
 * - Document CRUD (add, get, delete, bulk add)
 * - Search (query DSL, filters, sorting, aggregations, facets, highlights)
 */

export interface IndexInfo {
  name: string
  document_count: number
  fields: { name: string; type: string }[]
}

export interface SearchHit {
  document_id: string
  score: number
  fields: Record<string, unknown>
  highlights?: Record<string, string[]>
}

export interface SearchResponse {
  hits: SearchHit[]
  total_hits: number
  page: number
  page_size: number
  aggregations: Record<string, unknown>
  facet_distributions?: Record<string, Record<string, number>>
}

export interface SearchRequest {
  q?: string
  query?: { match: Record<string, string> } | { term: Record<string, string> } | { range: Record<string, { gte?: number; lte?: number }> }
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
