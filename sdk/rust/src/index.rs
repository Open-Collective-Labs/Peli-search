use crate::client::{url_encode, Client, Result};
use crate::types::*;

impl Client {
    // ── Indexes ──────────────────────────────────────────────────

    pub async fn list_indexes(&self) -> Result<Vec<String>> {
        let resp: ListIndexesResponse = self.get_("/indexes").await?;
        Ok(resp.indexes)
    }

    pub async fn get_index(&self, name: &str) -> Result<IndexInfo> {
        self.get_(&format!("/indexes/{}", url_encode(name)))
            .await
    }

    pub async fn create_index(&self, name: &str) -> Result<IndexCreatedResponse> {
        self.post_("/indexes", &CreateIndexRequest { name: name.to_string() })
            .await
    }

    pub async fn delete_index(&self, name: &str) -> Result<()> {
        let _: serde_json::Value =
            self.delete_(&format!("/indexes/{}", url_encode(name))).await?;
        Ok(())
    }

    // ── Documents ────────────────────────────────────────────────

    pub async fn add_document(
        &self,
        index: &str,
        id: &str,
        fields: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<DocumentCreatedResponse> {
        self.post_(
            &format!("/indexes/{}/documents", url_encode(index)),
            &AddDocumentRequest {
                id: id.to_string(),
                fields,
            },
        )
        .await
    }

    pub async fn get_document(
        &self,
        index: &str,
        id: &str,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        self.get_(&format!(
            "/indexes/{}/documents/{}",
            url_encode(index),
            url_encode(id),
        ))
        .await
    }

    pub async fn delete_document(&self, index: &str, id: &str) -> Result<()> {
        let _: serde_json::Value = self
            .delete_(&format!(
                "/indexes/{}/documents/{}",
                url_encode(index),
                url_encode(id),
            ))
            .await?;
        Ok(())
    }

    pub async fn bulk_add_documents(
        &self,
        index: &str,
        documents: Vec<AddDocumentRequest>,
    ) -> Result<BulkResponse> {
        self.post_(
            &format!("/indexes/{}/documents/bulk", url_encode(index)),
            &BulkAddRequest { documents },
        )
        .await
    }
}
