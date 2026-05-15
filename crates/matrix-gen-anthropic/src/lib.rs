//! Anthropic Messages API backend for the matrix-gen Oracle trait.

use matrix_gen_core::{Oracle, OracleError, OracleRequest};
use serde::{Deserialize, Serialize};

/// Default model. Haiku 4.5 is cost-efficient for the high call volume a
/// simulator generates; override via [`AnthropicOracle::with_model`].
pub const DEFAULT_MODEL: &str = "claude-haiku-4-5-20251001";

/// Anthropic Messages API version pin.
const API_VERSION: &str = "2023-06-01";
const API_URL: &str = "https://api.anthropic.com/v1/messages";

pub struct AnthropicOracle {
    api_key: String,
    model: String,
    max_tokens: u32,
    client: reqwest::Client,
}

impl AnthropicOracle {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: 1024,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self, AnthropicConfigError> {
        let api_key =
            std::env::var("ANTHROPIC_API_KEY").map_err(|_| AnthropicConfigError::MissingApiKey)?;
        Ok(Self::new(api_key))
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    pub fn with_endpoint(self, _endpoint: impl Into<String>) -> Self {
        // Anthropic endpoint is fixed; this is a placeholder for future proxy support.
        // Kept on the builder to avoid churn when proxies land. No-op today.
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AnthropicConfigError {
    #[error("ANTHROPIC_API_KEY not set")]
    MissingApiKey,
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<UserMessage<'a>>,
}

#[derive(Serialize)]
struct UserMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

impl Oracle for AnthropicOracle {
    fn complete(
        &self,
        request: &OracleRequest,
    ) -> impl Future<Output = Result<String, OracleError>> + Send {
        // Capture owned copies so the returned future doesn't borrow self.
        // Anthropic requests are cheap to copy; the network call dominates.
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let max_tokens = self.max_tokens;
        let client = self.client.clone();
        let system = request.system.clone();
        let user = request.user.clone();
        async move {
            let body = MessagesRequest {
                model: &model,
                max_tokens,
                system: system.as_deref(),
                messages: vec![UserMessage {
                    role: "user",
                    content: &user,
                }],
            };
            let resp = client
                .post(API_URL)
                .header("x-api-key", &api_key)
                .header("anthropic-version", API_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| OracleError::Backend(format!("send: {e}")))?;
            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(OracleError::Backend(format!("status {status}: {text}")));
            }
            let parsed: MessagesResponse = resp
                .json()
                .await
                .map_err(|e| OracleError::Backend(format!("decode: {e}")))?;
            let text = parsed
                .content
                .into_iter()
                .find_map(|b| match b {
                    ContentBlock::Text { text } => Some(text),
                    ContentBlock::Other => None,
                })
                .ok_or_else(|| OracleError::Backend("no text content".into()))?;
            Ok(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_constant_is_haiku_4_5() {
        assert_eq!(DEFAULT_MODEL, "claude-haiku-4-5-20251001");
    }

    #[test]
    fn builder_overrides_model_and_max_tokens() {
        let _: AnthropicOracle = AnthropicOracle::new("key")
            .with_model("claude-sonnet-4-6")
            .with_max_tokens(256);
    }
}
