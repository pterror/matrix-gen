use std::future::Future;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("embed backend: {0}")]
    Backend(String),
}

pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> impl Future<Output = Result<Vec<f32>, EmbedError>> + Send;
}

/// Deterministic hash-based embedder for tests. Same text → same vector.
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        assert!(dim > 0, "dim must be > 0");
        Self { dim }
    }
}

impl Embedder for HashEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> impl Future<Output = Result<Vec<f32>, EmbedError>> + Send {
        let dim = self.dim;
        let text = text.to_string();
        async move {
            // Hash each (token, axis) pair → deterministic float; produces stable
            // vectors that are sensitive to token content and position.
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut v = vec![0.0f32; dim];
            for token in text.split_whitespace() {
                for (axis, slot) in v.iter_mut().enumerate() {
                    let mut h = DefaultHasher::new();
                    token.hash(&mut h);
                    axis.hash(&mut h);
                    let raw = h.finish();
                    *slot += ((raw % 2000) as f32 / 1000.0) - 1.0;
                }
            }
            // L2-normalize so cosine == dot product.
            let norm: f32 = v
                .iter()
                .map(|x| x * x)
                .sum::<f32>()
                .sqrt()
                .max(f32::EPSILON);
            for x in &mut v {
                *x /= norm;
            }
            Ok(v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn embed_returns_correct_dim() {
        let e = HashEmbedder::new(16);
        let v = e.embed("hello").await.unwrap();
        assert_eq!(v.len(), 16);
    }

    #[tokio::test]
    async fn embed_is_deterministic() {
        let e = HashEmbedder::new(16);
        let v1 = e.embed("hello").await.unwrap();
        let v2 = e.embed("hello").await.unwrap();
        assert_eq!(v1, v2);
    }

    #[tokio::test]
    async fn embed_different_texts_differ() {
        let e = HashEmbedder::new(16);
        let v1 = e.embed("hello world").await.unwrap();
        let v2 = e.embed("goodbye moon").await.unwrap();
        assert_ne!(v1, v2);
    }

    #[tokio::test]
    async fn embed_is_l2_normalized() {
        let e = HashEmbedder::new(16);
        let v = e.embed("normalization check").await.unwrap();
        let sum_sq: f32 = v.iter().map(|x| x * x).sum();
        assert!((sum_sq - 1.0).abs() < 1e-5, "sum of squares = {sum_sq}");
    }
}
