import type { RequestFn, IndexInfo, IndexCreatedResponse } from "./types.js";
export declare class IndexesModule {
    private readonly request;
    constructor(request: RequestFn);
    list(): Promise<string[]>;
    get(name: string): Promise<IndexInfo>;
    create(name: string): Promise<IndexCreatedResponse>;
    delete(name: string): Promise<void>;
}
//# sourceMappingURL=indexes.d.ts.map