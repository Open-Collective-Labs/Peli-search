import { DocumentsModule } from "./documents.js";
import { PeliSearchError } from "./errors.js";
import { IndexesModule } from "./indexes.js";
import { SearchModule } from "./search.js";
const DEFAULT_HOST = "http://localhost:7700";
export class PeliSearchClient {
    request;
    searchModule;
    indexes;
    documents;
    constructor(opts = {}) {
        const baseUrl = (opts.host ?? DEFAULT_HOST).replace(/\/+$/, "");
        const headers = { "Content-Type": "application/json" };
        if (opts.apiKey) {
            headers["X-Api-Key"] = opts.apiKey;
        }
        this.request = (method, path, body) => PeliSearchClient.doFetch(baseUrl, headers, method, path, body);
        this.indexes = new IndexesModule(this.request);
        this.documents = new DocumentsModule(this.request);
        this.searchModule = new SearchModule(this.request);
    }
    // ── Indexes ──────────────────────────────────────────────────
    async createIndex(name) {
        return this.indexes.create(name);
    }
    async deleteIndex(name) {
        await this.indexes.delete(name);
    }
    async getIndex(name) {
        return this.indexes.get(name);
    }
    async listIndexes() {
        return this.indexes.list();
    }
    // ── Documents ────────────────────────────────────────────────
    async addDocument(index, id, fields) {
        return this.documents.add(index, id, fields);
    }
    async getDocument(index, id) {
        return this.documents.get(index, id);
    }
    async deleteDocument(index, id) {
        await this.documents.delete(index, id);
    }
    async bulkAddDocuments(index, documents) {
        return this.documents.bulkAdd(index, documents);
    }
    // ── Search ───────────────────────────────────────────────────
    async search(index, query) {
        return this.searchModule.search(index, query);
    }
    // ── Health ───────────────────────────────────────────────────
    async health() {
        await this.request("GET", "/health");
    }
    async ready() {
        await this.request("GET", "/ready");
    }
    async metrics() {
        return this.request("GET", "/metrics");
    }
    // ── Internal ─────────────────────────────────────────────────
    static async doFetch(baseUrl, headers, method, path, body) {
        const url = `${baseUrl}${path}`;
        const opts = {
            method,
            headers: { ...headers },
        };
        if (body !== undefined) {
            opts.body = JSON.stringify(body);
        }
        const res = await fetch(url, opts);
        const text = await res.text();
        if (!res.ok) {
            let msg;
            let parsed;
            try {
                parsed = JSON.parse(text);
                msg = parsed.error ?? res.statusText;
            }
            catch {
                msg = res.statusText;
            }
            throw new PeliSearchError(msg, res.status, parsed);
        }
        if (text.length === 0)
            return undefined;
        return JSON.parse(text);
    }
}
//# sourceMappingURL=client.js.map