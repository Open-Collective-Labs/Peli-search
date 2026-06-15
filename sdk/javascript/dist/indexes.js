export class IndexesModule {
    request;
    constructor(request) {
        this.request = request;
    }
    async list() {
        const body = await this.request("GET", "/indexes");
        return body.indexes;
    }
    async get(name) {
        return this.request("GET", `/indexes/${encodeURIComponent(name)}`);
    }
    async create(name) {
        return this.request("POST", "/indexes", { name });
    }
    async delete(name) {
        await this.request("DELETE", `/indexes/${encodeURIComponent(name)}`);
    }
}
//# sourceMappingURL=indexes.js.map