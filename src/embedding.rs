//! Embedding model abstraction

use async_trait::async_trait;
use std::sync::Arc;

use crate::config::EmbeddingConfig;
use crate::error::Result;

/// Create an embedder based on configuration
pub async fn create_embedder(config: &EmbeddingConfig) -> Result<Arc<dyn Embedder>> {
    match config.provider.as_str() {
        "openai" => Ok(Arc::new(OpenAIEmbedder::new(config)?)),
        "mock" => Ok(Arc::new(MockEmbedder::new(config.dimension))),
        _ => Err(crate::A3SError::Config(format!(
            "Unknown embedding provider: {}",
            config.provider
        ))),
    }
}

/// Embedder trait
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Embed a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed multiple texts in batch
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimension
    fn dimension(&self) -> usize;
}

/// OpenAI embedder implementation
pub struct OpenAIEmbedder {
    api_base: String,
    api_key: String,
    model: String,
    dimension: usize,
    batch_size: usize,
}

impl OpenAIEmbedder {
    pub fn new(config: &EmbeddingConfig) -> Result<Self> {
        let api_base = config
            .api_base
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| crate::A3SError::Config("No API key provided".to_string()))?;

        Ok(Self {
            api_base,
            api_key,
            model: config.model.clone(),
            dimension: config.dimension,
            batch_size: config.batch_size,
        })
    }
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        Ok(results.into_iter().next().unwrap())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let response = client
            .post(&format!("{}/embeddings", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::A3SError::Embedding(format!(
                "API error: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response.json().await?;

        let embeddings: Vec<Vec<f32>> = result["data"]
            .as_array()
            .ok_or_else(|| crate::A3SError::Embedding("Invalid response format".to_string()))?
            .iter()
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap() as f32)
                    .collect()
            })
            .collect();

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Mock embedder for testing (no API calls)
pub struct MockEmbedder {
    dimension: usize,
}

impl MockEmbedder {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl Embedder for MockEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Generate a deterministic embedding based on text hash
        let hash = text.bytes().fold(0u64, |acc, b| acc.wrapping_add(b as u64));
        let mut embedding = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            let val = ((hash.wrapping_add(i as u64) % 1000) as f32 / 1000.0) - 0.5;
            embedding.push(val);
        }
        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut embedding {
                *v /= norm;
            }
        }
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedder() {
        let embedder = MockEmbedder::new(128);
        let embedding = embedder.embed("test text").await.unwrap();

        assert_eq!(embedding.len(), 128);

        // Check normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_mock_embedder_deterministic() {
        let embedder = MockEmbedder::new(64);
        let e1 = embedder.embed("same text").await.unwrap();
        let e2 = embedder.embed("same text").await.unwrap();

        assert_eq!(e1, e2);
    }

    #[tokio::test]
    async fn test_mock_embedder_batch() {
        let embedder = MockEmbedder::new(64);
        let texts = vec!["text1".to_string(), "text2".to_string()];
        let embeddings = embedder.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 64);
        assert_eq!(embeddings[1].len(), 64);
    }

    #[tokio::test]
    async fn test_create_mock_embedder() {
        let config = EmbeddingConfig {
            provider: "mock".to_string(),
            api_base: None,
            api_key: None,
            model: "mock".to_string(),
            dimension: 128,
            batch_size: 32,
        };

        let embedder = create_embedder(&config).await.unwrap();
        assert_eq!(embedder.dimension(), 128);
    }
}
