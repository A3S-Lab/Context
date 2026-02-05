//! Jina Reranker API implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{RerankDocument, RerankResult, Reranker};
use crate::config::RerankConfig;
use crate::error::Result;

const DEFAULT_API_BASE: &str = "https://api.jina.ai/v1";
const DEFAULT_MODEL: &str = "jina-reranker-v2-base-multilingual";

/// Jina reranker using the Jina Rerank API
pub struct JinaReranker {
    api_base: String,
    api_key: String,
    model: String,
}

impl JinaReranker {
    pub fn new(config: &RerankConfig) -> Result<Self> {
        let api_base = config
            .api_base
            .clone()
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("JINA_API_KEY").ok())
            .ok_or_else(|| crate::A3SError::Config("Jina API key not provided".to_string()))?;

        let model = config
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());

        Ok(Self {
            api_base,
            api_key,
            model,
        })
    }
}

#[derive(Serialize)]
struct JinaRerankRequest {
    query: String,
    documents: Vec<String>,
    model: String,
    top_n: usize,
}

#[derive(Deserialize)]
struct JinaRerankResponse {
    results: Vec<JinaRerankResult>,
}

#[derive(Deserialize)]
struct JinaRerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait]
impl Reranker for JinaReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
        top_n: usize,
    ) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // Store document IDs for later mapping
        let doc_ids: Vec<String> = documents.iter().map(|d| d.id.clone()).collect();
        let doc_texts: Vec<String> = documents.into_iter().map(|d| d.text).collect();

        let request = JinaRerankRequest {
            query: query.to_string(),
            documents: doc_texts,
            model: self.model.clone(),
            top_n,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&format!("{}/rerank", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::A3SError::Rerank(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(crate::A3SError::Rerank(format!(
                "Jina API error {}: {}",
                status, body
            )));
        }

        let result: JinaRerankResponse = response
            .json()
            .await
            .map_err(|e| crate::A3SError::Rerank(format!("Failed to parse response: {}", e)))?;

        let results = result
            .results
            .into_iter()
            .map(|r| RerankResult {
                id: doc_ids.get(r.index).cloned().unwrap_or_default(),
                index: r.index,
                score: r.relevance_score,
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jina_reranker_new_without_key() {
        std::env::remove_var("JINA_API_KEY");
        let config = RerankConfig {
            provider: "jina".to_string(),
            api_base: None,
            api_key: None,
            model: None,
            top_n: None,
        };
        let result = JinaReranker::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_jina_reranker_new_with_config_key() {
        let config = RerankConfig {
            provider: "jina".to_string(),
            api_base: Some("https://custom.api".to_string()),
            api_key: Some("test-key".to_string()),
            model: Some("custom-model".to_string()),
            top_n: Some(5),
        };
        let reranker = JinaReranker::new(&config).unwrap();
        assert_eq!(reranker.api_base, "https://custom.api");
        assert_eq!(reranker.api_key, "test-key");
        assert_eq!(reranker.model, "custom-model");
    }

    #[test]
    fn test_jina_reranker_new_with_env_key() {
        std::env::set_var("JINA_API_KEY", "env-test-key");
        let config = RerankConfig {
            provider: "jina".to_string(),
            api_base: None,
            api_key: None,
            model: None,
            top_n: None,
        };
        let reranker = JinaReranker::new(&config).unwrap();
        assert_eq!(reranker.api_key, "env-test-key");
        assert_eq!(reranker.model, DEFAULT_MODEL);
        std::env::remove_var("JINA_API_KEY");
    }

    #[tokio::test]
    async fn test_jina_reranker_empty_documents() {
        let config = RerankConfig {
            provider: "jina".to_string(),
            api_base: None,
            api_key: Some("test-key".to_string()),
            model: None,
            top_n: None,
        };
        let reranker = JinaReranker::new(&config).unwrap();
        let results = reranker.rerank("query", vec![], 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_jina_reranker_live() {
        let config = RerankConfig {
            provider: "jina".to_string(),
            api_base: None,
            api_key: None, // Uses JINA_API_KEY env var
            model: None,
            top_n: None,
        };
        let reranker = JinaReranker::new(&config).unwrap();
        let documents = vec![
            RerankDocument {
                id: "doc1".to_string(),
                text: "The capital of France is Paris.".to_string(),
            },
            RerankDocument {
                id: "doc2".to_string(),
                text: "Python is a programming language.".to_string(),
            },
            RerankDocument {
                id: "doc3".to_string(),
                text: "Paris is known for the Eiffel Tower.".to_string(),
            },
        ];

        let results = reranker
            .rerank("What is the capital of France?", documents, 2)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        // The most relevant document should be about Paris/France
        assert!(results[0].score > results[1].score);
    }
}
