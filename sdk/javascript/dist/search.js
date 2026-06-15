export class SearchModule {
    request;
    constructor(request) {
        this.request = request;
    }
    async search(index, request) {
        return this.request("POST", `/indexes/${encodeURIComponent(index)}/search`, request);
    }
}
//# sourceMappingURL=search.js.map