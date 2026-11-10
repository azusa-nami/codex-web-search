use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use base64::Engine;
use reqwest::header::AUTHORIZATION;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde_json::Value;

/// Caller-owned bearer credentials. No login, persistence or token refresh is performed.
/// Deliberately does not implement Debug or Serialize.
pub struct SearchAuth {
    pub(crate) headers: HeaderMap,
}

impl SearchAuth {
    pub fn from_access_token(token: &str, account_id: Option<&str>) -> Result<Self> {
        let token = token.trim();
        ensure!(!token.is_empty(), "access token is empty");
        // Decode only for the routing hint, never as proof of authentication.
        // The remote service verifies the token. Opaque bearer tokens work too.
        let claims = token
            .split('.')
            .nth(/*n*/ 1)
            .and_then(|payload| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(payload)
                    .ok()
            })
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        let account_id = account_id.or_else(|| {
            claims.as_ref()?["https://api.openai.com/auth"]["chatgpt_account_id"].as_str()
        });
        let mut headers = HeaderMap::new();
        let mut bearer = HeaderValue::from_str(&format!("Bearer {token}"))
            .context("access token is not a valid HTTP header value")?;
        bearer.set_sensitive(/*val*/ true);
        headers.insert(AUTHORIZATION, bearer);
        if let Some(account_id) = account_id {
            ensure!(!account_id.trim().is_empty(), "account ID is empty");
            headers.insert(
                "chatgpt-account-id",
                HeaderValue::from_str(account_id).context("invalid account ID")?,
            );
        }
        Ok(Self { headers })
    }

    /// Reads the token subset of Codex's auth.json; ignores refresh/id tokens.
    pub fn from_auth_json(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 256 * 1024, "auth file exceeds 256 KiB");
        let auth: Value = serde_json::from_slice(bytes).context("invalid auth JSON")?;
        let token = auth["tokens"]["access_token"]
            .as_str()
            .context("auth file has no ChatGPT access token")?;
        Self::from_access_token(token, auth["tokens"]["account_id"].as_str())
    }
}
