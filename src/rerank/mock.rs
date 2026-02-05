//! Mock reranker for testing

use async_trait::async_trait;

use super::{RerankDocument, RerankResult, Reranker};
use crate::error::Result;

/// Mock reranker that returns documents in original order with synthetic scores
pub struct MockReranker;

impl MockReranker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for MockReranker {
    async fn rerank(
        &self,
        _query: &str,
        documents: Vec<RerankDocument>,
        top_n: usize,
    ) -> Result<Vec<RerankResult>> {
        let mut results: Vec<RerankResult> = documents
            .into_iter()
            .enumerate()
            .map(|(index, doc)| {
                // Generate a deterministic score based on document content
                let hash = doc
                    .text
                    .bytes()
                    .fold(0u64, |acc, b| acc.wrapping_add(b as u64));
                let score = (hash % 100) as f32 / 100.0;
                RerankResult {
                    id: doc.id,
                    index,
                    score,
                }
            })
            .collect();

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

    #[tokio::test]
    async fn test_mock_reranker_basic() {
        let reranker = MockReranker::new();
        let documents = vec![
            RerankDocument {
                id: "doc1".to_string(),
                text: "first document".to_string(),
            },
            RerankDocument {
                id: "doc2".to_string(),
                text: "second document".to_string(),
            },
        ];

        let results = reranker.rerank("test query", documents, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        // All results should have valid scores
        for result in &results {
            assert!(result.score >= 0.0 && result.score <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_mock_reranker_top_n() {
        let reranker = MockReranker::new();
        let documents = vec![
            RerankDocument {
                id: "doc1".to_string(),
                text: "first".to_string(),
            },
            RerankDocument {
                id: "doc2".to_string(),
                text: "second".to_string(),
            },
            RerankDocument {
                id: "doc3".to_string(),
                text: "third".to_string(),
            },
        ];

        let results = reranker.rerank("query", documents, 2).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_reranker_empty() {
        let reranker = MockReranker::new();
        let documents: Vec<RerankDocument> = vec![];

        let results = reranker.rerank("query", documents, 5).await.unwrap();

        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_mock_reranker_sorted() {
        let reranker = MockReranker::new();
        let documents = vec![
            RerankDocument {
                id: "doc1".to_string(),
                text: "aaa".to_string(),
            },
            RerankDocument {
                id: "doc2".to_string(),
                text: "zzz".to_string(),
            },
        ];

        let results = reranker.rerank("query", documents, 2).await.unwrap();

        // Results should be sorted by score descending
        if results.len() >= 2 {
            assert!(results[0].score >= results[1].score);
        }
    }

    #[test]
    fn test_mock_reranker_default() {
        let reranker = MockReranker::default();
        // Just verify it can be created via Default
        let _ = reranker;
    }
}
