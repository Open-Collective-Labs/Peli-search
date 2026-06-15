use crate::client::{url_encode, Client, Result};
use crate::types::*;

impl Client {
    // ── Search ───────────────────────────────────────────────────

    pub async fn search(
        &self,
        index: &str,
        request: &SearchRequest,
    ) -> Result<SearchResponse> {
        self.post_(
            &format!("/indexes/{}/search", url_encode(index)),
            request,
        )
        .await
    }
}
