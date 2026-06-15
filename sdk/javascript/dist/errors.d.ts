export declare class PeliSearchError extends Error {
    readonly status: number;
    readonly body: unknown;
    constructor(message: string, status: number, body?: unknown);
}
export declare function isPeliSearchError(err: unknown): err is PeliSearchError;
//# sourceMappingURL=errors.d.ts.map