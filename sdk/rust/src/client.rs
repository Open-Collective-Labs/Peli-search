use reqwest::{Client as HttpClient, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::*;

#[derive(Debug, thiserror::Error)]
pub enum PeliSearchError {
    #[error("HTTP error: {status} — {message}")]
    Http { status: u16, message: String },
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("URL parse: {0}")]
    Url(#[from] url::ParseError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, PeliSearchError>;

pub struct Client {
    pub(crate) http: HttpClient,
    pub(crate) base: Url,
}

impl Client {
    pub fn new(host: &str, port: u16) -> Result<Self> {
        Self::from_url(&format!("http://{host}:{port}"))
    }

    pub fn from_url(base_url: &str) -> Result<Self> {
        let base = Url::parse(base_url)?;
        Ok(Self {
            http: HttpClient::new(),
            base,
        })
    }

    pub fn with_http_client(http: HttpClient, base: Url) -> Self {
        Self { http, base }
    }

    // ── Health ───────────────────────────────────────────────────

    pub async fn health(&self) -> Result<()> {
        self.get_::<serde_json::Value>("/health").await?;
        Ok(())
    }

    pub async fn ready(&self) -> Result<()> {
        self.get_::<serde_json::Value>("/ready").await?;
        Ok(())
    }

    // ── Internal ─────────────────────────────────────────────────

    pub(crate) async fn get_<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send::<T>("GET", path).await
    }

    pub(crate) async fn post_<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T> {
        let url = self.base.join(path.trim_start_matches('/'))?;
        let resp = self
            .http
            .request(reqwest::Method::POST, url)
            .json(body)
            .send()
            .await?;
        self.parse_response(resp).await
    }

    pub(crate) async fn delete_<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.send::<T>("DELETE", path).await
    }

    async fn send<T: DeserializeOwned>(&self, method: &str, path: &str) -> Result<T> {
        let url = self.base.join(path.trim_start_matches('/'))?;
        let m = reqwest::Method::from_bytes(method.as_bytes()).unwrap();
        let resp = self.http.request(m, url).send().await?;
        self.parse_response(resp).await
    }

    async fn parse_response<T: DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T> {
        let status = resp.status();
        if !status.is_success() {
            let msg = match resp.json::<ErrorResponse>().await {
                Ok(er) => er.error,
                Err(_) => status.to_string(),
            };
            return Err(PeliSearchError::Http {
                status: status.as_u16(),
                message: msg,
            });
        }

        if status == StatusCode::NO_CONTENT {
            serde_json::from_str("null").map_err(Into::into)
        } else {
            resp.json().await.map_err(Into::into)
        }
    }
}

pub(crate) fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
