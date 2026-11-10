//! Independent client for Codex's standalone alpha/search endpoint.
//! This endpoint is not the public Responses API web_search tool.
mod auth;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use anyhow::ensure;
pub use auth::SearchAuth;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex/";
const MAX_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Default)]
pub enum SearchMode {
    Cached,
    Indexed,
    #[default]
    Live,
}

/// Reuse id across search/open/find calls to keep server-side references valid.
pub struct SearchRequest {
    pub id: String,
    pub model: String,
    /// The same JSON object accepted by web.run, including search_query/open/find.
    pub commands: Value,
    pub input: Option<Value>,
    pub mode: SearchMode,
    pub max_output_tokens: u64,
}

impl SearchRequest {
    pub fn new(id: String, model: String, commands: Value) -> Self {
        Self {
            id,
            model,
            commands,
            input: None,
            mode: SearchMode::Live,
            max_output_tokens: 10_000,
        }
    }

    pub fn body(&self) -> Value {
        let access = match self.mode {
            SearchMode::Cached => json!(false),
            SearchMode::Indexed => json!("indexed"),
            SearchMode::Live => json!(true),
        };
        let mut body = json!({"id":self.id,"model":self.model,"commands":self.commands,
            "settings":{"allowed_callers":["direct"],"external_web_access":access},
            "max_output_tokens":self.max_output_tokens});
        if let Some(input) = &self.input {
            body["input"] = input.clone();
        }
        body
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SearchResponse {
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Value>>,
}

pub struct SearchClient {
    http: reqwest::Client,
    endpoint: Url,
}

impl SearchClient {
    pub fn new(auth: SearchAuth) -> Result<Self> {
        Self::with_base_url(auth, DEFAULT_BASE_URL)
    }

    /// Base URL of a compatible endpoint. HTTP is supported only on loopback.
    pub fn with_base_url(auth: SearchAuth, base_url: &str) -> Result<Self> {
        let base = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))
            .map_err(|_| anyhow::anyhow!("invalid search base URL"))?;
        ensure!(
            base.scheme() == "https"
                || (base.scheme() == "http"
                    && base.host_str().is_some_and(|host| {
                        host == "localhost"
                            || host
                                .trim_matches(&['[', ']'][..])
                                .parse::<std::net::IpAddr>()
                                .is_ok_and(|ip| ip.is_loopback())
                    })),
            "search base URL must use HTTPS (or loopback HTTP)"
        );
        ensure!(
            base.username().is_empty()
                && base.password().is_none()
                && base.query().is_none()
                && base.fragment().is_none(),
            "base URL must not contain credentials, a query or a fragment"
        );
        Ok(Self {
            http: reqwest::Client::builder()
                .default_headers(auth.headers)
                .user_agent("codex-web-search/0.1.0")
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(/*secs*/ 60))
                .build()?,
            endpoint: base.join("alpha/search")?,
        })
    }

    pub async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        ensure!(!request.id.trim().is_empty(), "session ID is empty");
        ensure!(!request.model.trim().is_empty(), "model is empty");
        ensure!(
            request.commands.is_object(),
            "commands must be a JSON object"
        );
        ensure!(
            request.max_output_tokens > 0,
            "max_output_tokens must be positive"
        );
        let body = serde_json::to_vec(&request.body())?;
        ensure!(body.len() <= MAX_BYTES, "search request exceeds 256 KiB");
        // Do not replay POSTs automatically: the service may already have run the search.
        let mut response = self
            .http
            .post(self.endpoint.clone())
            .header("originator", "codex_web_search")
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            bail!(
                "search returned HTTP 401; update the access token or log in again (no automatic refresh)"
            );
        }
        ensure!(status.is_success(), "search returned HTTP {status}");
        if let Some(length) = response.content_length() {
            ensure!(
                length <= MAX_BYTES as u64,
                "search response exceeds 256 KiB"
            );
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= MAX_BYTES,
                "search response exceeds 256 KiB"
            );
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).context("invalid search response JSON")
    }
}
