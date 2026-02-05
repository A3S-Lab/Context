//! Reranking module for improving retrieval quality
//!
//! This module provides reranking capabilities to reorder search results
//! using specialized reranking models after initial vector search.

mod cohere;
mod jina;
mod mock;
mod openai;

pub use cohere::CohereReranker;
pub use jina::JinaReranker;
pub use mock::MockReranker;
pub use openai::OpenAIReranker;

use async_trait::async_trait;
use std::sync::Arc;

use crate::config::RerankConfig;
use crate::error::Result;

/// Document to be reranked
#[derive(Debug, Clone)]
pub struct RerankDocument {
    /// Unique identifier for the document
    pub id: String,
    /// Text content to be scored against the query
    pub text: String,
}

/// Result of reranking a document
#[derive(Debug, Clone)]
pub struct RerankResult {
    /// Document identifier
    pub id: String,
    /// Original index in the input list
    pub index: usize,
    /// Relevance score from the reranker (higher is more relevant)
    pub score: f32,
}

/// Reranker trait for reordering search results
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank documents based on relevance to the query
    ///
    /// # Arguments
    /// * `query` - The search query
    /// * `documents` - Documents to rerank
    /// * `top_n` - Number of top results to return
    ///
    /// # Returns
    /// Reranked results sorted by relevance score (descending)
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
        top_n: usize,
    ) -> Result<Vec<RerankResult>>;
}

/// Create a reranker based on configuration
pub fn create_reranker(config: &RerankConfig) -> Result<Arc<dyn Reranker>> {
    match config.provider.as_str() {
        "mock" => Ok(Arc::new(MockReranker::new())),
        "cohere" => Ok(Arc::new(CohereReranker::new(config)?)),
        "jina" => Ok(Arc::new(JinaReranker::new(config)?)),
        "openai" => Ok(Arc::new(OpenAIReranker::new(config)?)),
        _ => Err(crate::A3SError::Config(format!(
            "Unknown rerank provider: {}",
            config.provider
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rerank_document() {
        let doc = RerankDocument {
            id: "doc1".to_string(),
            text: "test content".to_string(),
        };
        assert_eq!(doc.id, "doc1");
        assert_eq!(doc.text, "test content");
    }

    #[test]
    fn test_rerank_result() {
        let result = RerankResult {
            id: "doc1".to_string(),
            index: 0,
            score: 0.95,
        };
        assert_eq!(result.id, "doc1");
        assert_eq!(result.index, 0);
        assert!((result.score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_create_mock_reranker() {
        let config = RerankConfig::default();
        let reranker = create_reranker(&config);
        assert!(reranker.is_ok());
    }

    #[test]
    fn test_create_unknown_reranker() {
        let config = RerankConfig {
            provider: "unknown".to_string(),
            ..Default::default()
        };
        let result = create_reranker(&config);
        assert!(result.is_err());
    }
}
