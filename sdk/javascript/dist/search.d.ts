import type { RequestFn, SearchRequest, SearchResponse } from "./types.js";
export declare class SearchModule {
    private readonly request;
    constructor(request: RequestFn);
    search(index: string, request: SearchRequest): Promise<SearchResponse>;
}
//# sourceMappingURL=search.d.ts.map