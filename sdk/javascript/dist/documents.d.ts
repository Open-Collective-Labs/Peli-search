import type { RequestFn, DocumentCreatedResponse, BulkResponse } from "./types.js";
export interface AddDocumentPayload {
    id: string;
    fields: Record<string, unknown>;
}
export declare class DocumentsModule {
    private readonly request;
    constructor(request: RequestFn);
    add(index: string, id: string, fields: Record<string, unknown>): Promise<DocumentCreatedResponse>;
    get(index: string, id: string): Promise<Record<string, unknown>>;
    delete(index: string, id: string): Promise<void>;
    bulkAdd(index: string, documents: AddDocumentPayload[]): Promise<BulkResponse>;
}
//# sourceMappingURL=documents.d.ts.map