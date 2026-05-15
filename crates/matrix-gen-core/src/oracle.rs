use std::collections::VecDeque;
use std::future::Future;
use std::sync::Mutex;

/// A single prompt to the LLM oracle.
#[derive(Debug, Clone)]
pub struct OracleRequest {
    pub system: Option<String>,
    pub user: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("oracle exhausted")]
    Exhausted,
    #[error("oracle backend: {0}")]
    Backend(String),
}

/// Stateless LLM call — takes a request, returns generated text.
pub trait Oracle: Send + Sync {
    fn complete(
        &self,
        request: &OracleRequest,
    ) -> impl Future<Output = Result<String, OracleError>> + Send;
}

/// Returns the user prompt verbatim. For sanity tests.
pub struct EchoOracle;

impl Oracle for EchoOracle {
    fn complete(
        &self,
        request: &OracleRequest,
    ) -> impl Future<Output = Result<String, OracleError>> + Send {
        let response = request.user.clone();
        async move { Ok(response) }
    }
}

/// Returns canned responses in order; exhausted when the queue drains.
pub struct ScriptedOracle {
    responses: Mutex<VecDeque<String>>,
}

impl ScriptedOracle {
    pub fn new<I: IntoIterator<Item = S>, S: Into<String>>(responses: I) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().map(Into::into).collect()),
        }
    }
}

impl Oracle for ScriptedOracle {
    fn complete(
        &self,
        _request: &OracleRequest,
    ) -> impl Future<Output = Result<String, OracleError>> + Send {
        let result = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(OracleError::Exhausted);
        async move { result }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_oracle_returns_user_prompt() {
        let oracle = EchoOracle;
        let req = OracleRequest {
            system: None,
            user: "hello world".into(),
        };
        assert_eq!(oracle.complete(&req).await.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn scripted_oracle_returns_in_order() {
        let oracle = ScriptedOracle::new(["first", "second"]);
        let req = OracleRequest {
            system: None,
            user: "ignored".into(),
        };
        assert_eq!(oracle.complete(&req).await.unwrap(), "first");
        assert_eq!(oracle.complete(&req).await.unwrap(), "second");
    }

    #[tokio::test]
    async fn scripted_oracle_exhausted_after_queue_drains() {
        let oracle = ScriptedOracle::new(["only"]);
        let req = OracleRequest {
            system: None,
            user: "ignored".into(),
        };
        oracle.complete(&req).await.unwrap();
        let err = oracle.complete(&req).await.unwrap_err();
        assert!(matches!(err, OracleError::Exhausted));
    }
}
