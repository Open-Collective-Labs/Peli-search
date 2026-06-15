export class PeliSearchError extends Error {
    status;
    body;
    constructor(message, status, body) {
        super(message);
        this.name = "PeliSearchError";
        this.status = status;
        this.body = body;
    }
}
export function isPeliSearchError(err) {
    return err instanceof PeliSearchError;
}
//# sourceMappingURL=errors.js.map