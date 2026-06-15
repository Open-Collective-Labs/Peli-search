export class DocumentsModule {
    request;
    constructor(request) {
        this.request = request;
    }
    async add(index, id, fields) {
        return this.request("POST", `/indexes/${encodeURIComponent(index)}/documents`, { id, fields });
    }
    async get(index, id) {
        return this.request("GET", `/indexes/${encodeURIComponent(index)}/documents/${encodeURIComponent(id)}`);
    }
    async delete(index, id) {
        await this.request("DELETE", `/indexes/${encodeURIComponent(index)}/documents/${encodeURIComponent(id)}`);
    }
    async bulkAdd(index, documents) {
        return this.request("POST", `/indexes/${encodeURIComponent(index)}/documents/bulk`, { documents });
    }
}
//# sourceMappingURL=documents.js.map