import { DocumentsModule } from "./documents.js";
import { IndexesModule } from "./indexes.js";
import type { ClientOptions, SearchRequest, SearchResponse } from "./types.js";
export declare class PeliSearchClient {
    private readonly request;
    private readonly searchModule;
    readonly indexes: IndexesModule;
    readonly documents: DocumentsModule;
    constructor(opts?: ClientOptions);
    createIndex(name: string): Promise<import("./types.js").IndexCreatedResponse>;
    deleteIndex(name: string): Promise<void>;
    getIndex(name: string): Promise<import("./types.js").IndexInfo>;
    listIndexes(): Promise<string[]>;
    addDocument(index: string, id: string, fields: Record<string, unknown>): Promise<import("./types.js").DocumentCreatedResponse>;
    getDocument(index: string, id: string): Promise<Record<string, unknown>>;
    deleteDocument(index: string, id: string): Promise<void>;
    bulkAddDocuments(index: string, documents: {
        id: string;
        fields: Record<string, unknown>;
    }[]): Promise<import("./types.js").BulkResponse>;
    search(index: string, query: SearchRequest): Promise<SearchResponse>;
    health(): Promise<void>;
    ready(): Promise<void>;
    private static doFetch;
}
//# sourceMappingURL=client.d.ts.map