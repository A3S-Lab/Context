//! OpenAI pointwise reranker implementation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{RerankDocument, RerankResult, Reranker};
use crate::config::RerankConfig;
use crate::error::Result;

const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

/// OpenAI reranker using pointwise scoring via chat completions
pub struct OpenAIReranker {
    api_base: String,
    api_key: String,
    model: String,
}

impl OpenAIReranker {
    pub fn new(config: &RerankConfig) -> Result<Self> {
        let api_base = config
            .api_base
            .clone()
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| crate::A3SError::Config("OpenAI API key not provided".to_string()))?;

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

    async fn score_document(&self, query: &str, document: &str) -> Result<f32> {
        let prompt = format!(
            "Rate the relevance of the following document to the query on a scale of 0 to 10.\n\n\
             Query: {}\n\n\
             Document: {}\n\n\
             Respond with ONLY a number between 0 and 10, nothing else.",
            query, document
        );

        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.0,
            max_tokens: 10,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/chat/completions", self.api_base))
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
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let result: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| crate::A3SError::Rerank(format!("Failed to parse response: {}", e)))?;

        let content = result
            .choices
            .first()
            .map(|c| c.message.content.trim())
            .unwrap_or("0");

        // Parse the score, defaulting to 0 if parsing fails
        let score: f32 = content.parse().unwrap_or(0.0);

        // Normalize to 0-1 range
        Ok(score / 10.0)
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[async_trait]
impl Reranker for OpenAIReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<RerankDocument>,
        top_n: usize,
    ) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        // Score each document (could be parallelized for better performance)
        let mut results = Vec::with_capacity(documents.len());
        for (index, doc) in documents.iter().enumerate() {
            let score = self.score_document(query, &doc.text).await?;
            results.push(RerankResult {
                id: doc.id.clone(),
                index,
                score,
            });
        }

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate to top_n
        results.truncate(top_n);

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_reranker_new_without_key() {
        std::env::remove_var("OPENAI_API_KEY");
        let config = RerankConfig {
            provider: "openai".to_string(),
            api_base: None,
            api_key: None,
            model: None,
            top_n: None,
        };
        let result = OpenAIReranker::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_openai_reranker_new_with_config_key() {
        let config = RerankConfig {
            provider: "openai".to_string(),
            api_base: Some("https://custom.api".to_string()),
            api_key: Some("test-key".to_string()),
            model: Some("gpt-4".to_string()),
            top_n: Some(5),
        };
        let reranker = OpenAIReranker::new(&config).unwrap();
        assert_eq!(reranker.api_base, "https://custom.api");
        assert_eq!(reranker.api_key, "test-key");
        assert_eq!(reranker.model, "gpt-4");
    }

    #[test]
    fn test_openai_reranker_new_with_env_key() {
        std::env::set_var("OPENAI_API_KEY", "env-test-key");
        let config = RerankConfig {
            provider: "openai".to_string(),
            api_base: None,
            api_key: None,
            model: None,
            top_n: None,
        };
        let reranker = OpenAIReranker::new(&config).unwrap();
        assert_eq!(reranker.api_key, "env-test-key");
        assert_eq!(reranker.model, DEFAULT_MODEL);
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[tokio::test]
    async fn test_openai_reranker_empty_documents() {
        let config = RerankConfig {
            provider: "openai".to_string(),
            api_base: None,
            api_key: Some("test-key".to_string()),
            model: None,
            top_n: None,
        };
        let reranker = OpenAIReranker::new(&config).unwrap();
        let results = reranker.rerank("query", vec![], 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_openai_reranker_live() {
        let config = RerankConfig {
            provider: "openai".to_string(),
            api_base: None,
            api_key: None, // Uses OPENAI_API_KEY env var
            model: None,
            top_n: None,
        };
        let reranker = OpenAIReranker::new(&config).unwrap();
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
