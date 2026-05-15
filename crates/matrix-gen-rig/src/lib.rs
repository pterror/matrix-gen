use matrix_gen_core::{EmbedError, Embedder, Oracle, OracleError, OracleRequest};
use rig::completion::CompletionModel;
use rig::embeddings::EmbeddingModel;

pub use rig;

// ─── RigOracle ────────────────────────────────────────────────────────────────

pub struct RigOracle<M: CompletionModel> {
    model: M,
    preamble: Option<String>,
}

impl<M: CompletionModel> RigOracle<M> {
    pub fn new(model: M) -> Self {
        Self {
            model,
            preamble: None,
        }
    }

    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }
}

impl<M: CompletionModel + Send + Sync + 'static> Oracle for RigOracle<M> {
    async fn complete(&self, request: &OracleRequest) -> Result<String, OracleError> {
        // Build the system prompt by joining preamble and per-request system if present.
        let system: Option<String> = match (&self.preamble, &request.system) {
            (Some(p), Some(s)) => Some(format!("{p}\n\n{s}")),
            (Some(p), None) => Some(p.clone()),
            (None, Some(s)) => Some(s.clone()),
            (None, None) => None,
        };

        let mut builder = self.model.completion_request(request.user.clone());
        if let Some(s) = system {
            builder = builder.preamble(s);
        }
        let req = builder.build();
        let response = self
            .model
            .completion(req)
            .await
            .map_err(|e| OracleError::Backend(format!("{e}")))?;
        let text = response
            .choice
            .into_iter()
            .filter_map(|c| match c {
                rig::completion::AssistantContent::Text(t) => Some(t.text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        Ok(text)
    }
}

// ─── RigEmbedder ─────────────────────────────────────────────────────────────

pub struct RigEmbedder<E: EmbeddingModel> {
    model: E,
}

impl<E: EmbeddingModel> RigEmbedder<E> {
    pub fn new(model: E) -> Self {
        Self { model }
    }
}

impl<E: EmbeddingModel + Send + Sync + 'static> Embedder for RigEmbedder<E> {
    fn dim(&self) -> usize {
        self.model.ndims()
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let embeddings = self
            .model
            .embed_texts(vec![text.to_owned()])
            .await
            .map_err(|e| EmbedError::Backend(format!("{e}")))?;
        let vec = embeddings
            .into_iter()
            .next()
            .map(|e| e.vec.into_iter().map(|v| v as f32).collect())
            .unwrap_or_default();
        Ok(vec)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Confirm RigOracle<M> builder compiles and returns Self for any CompletionModel M.
    #[allow(dead_code)]
    fn _oracle_builder_compiles<M: CompletionModel>(m: M) {
        let _: RigOracle<M> = RigOracle::new(m).with_preamble("hi");
    }

    // Confirm RigEmbedder<E> builder compiles for any EmbeddingModel E.
    #[allow(dead_code)]
    fn _embedder_builder_compiles<E: EmbeddingModel>(e: E) {
        let _: RigEmbedder<E> = RigEmbedder::new(e);
    }
}
