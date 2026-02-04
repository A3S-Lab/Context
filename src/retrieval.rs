//! Hierarchical retrieval system

use std::sync::Arc;
use std::time::Instant;

use crate::config::RetrievalConfig;
use crate::core::Namespace;
use crate::embedding::Embedder;
use crate::error::Result;
use crate::pathway::Pathway;
use crate::storage::StorageBackend;
use crate::{MatchedNode, QueryOptions, QueryResult};

/// Hierarchical retriever for semantic search
pub struct Retriever {
    storage: Arc<dyn StorageBackend>,
    embedder: Arc<dyn Embedder>,
    config: RetrievalConfig,
}

impl Retriever {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        embedder: Arc<dyn Embedder>,
        config: &RetrievalConfig,
    ) -> Self {
        Self {
            storage,
            embedder,
            config: config.clone(),
        }
    }

    /// Search for relevant context
    pub async fn search(
        &self,
        query: &str,
        options: Option<QueryOptions>,
    ) -> Result<QueryResult> {
        let options = options.unwrap_or_default();

        // Generate query embedding
        let embed_start = Instant::now();
        let query_vector = self.embedder.embed(query).await?;
        let embed_time = embed_start.elapsed().as_millis() as u64;

        let search_start = Instant::now();

        // Determine search parameters
        let limit = options.limit.unwrap_or(self.config.default_limit);
        let threshold = options.threshold.unwrap_or(self.config.score_threshold);

        // Perform vector search
        let candidates = self
            .storage
            .search_vector(&query_vector, options.namespace, limit * 3, threshold)
            .await?;

        // If hierarchical search is enabled, explore directories
        let mut results = if self.config.hierarchical {
            self.hierarchical_search(&query_vector, &candidates, limit, threshold)
                .await?
        } else {
            self.flat_search(&candidates, limit).await?
        };

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        let search_time = search_start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            matches: results,
            total_searched: candidates.len(),
            query_embedding_time_ms: embed_time,
            search_time_ms: search_time,
        })
    }

    async fn flat_search(
        &self,
        candidates: &[(Pathway, f32)],
        limit: usize,
    ) -> Result<Vec<MatchedNode>> {
        let mut results = Vec::new();

        for (pathway, score) in candidates.iter().take(limit) {
            let node = self.storage.get(pathway).await?;

            results.push(MatchedNode {
                pathway: pathway.clone(),
                node_kind: node.kind,
                score: *score,
                brief: node.digest.brief,
                summary: Some(node.digest.summary),
                content: None,
                highlights: Vec::new(),
            });
        }

        Ok(results)
    }

    async fn hierarchical_search(
        &self,
        query_vector: &[f32],
        initial_candidates: &[(Pathway, f32)],
        _limit: usize,
        threshold: f32,
    ) -> Result<Vec<MatchedNode>> {
        let mut results = Vec::new();
        let mut explored_dirs = std::collections::HashSet::new();

        // First pass: collect initial results and identify promising directories
        for (pathway, score) in initial_candidates {
            if *score < threshold {
                continue;
            }

            let node = self.storage.get(pathway).await?;

            if node.is_directory {
                explored_dirs.insert(pathway.clone());
            } else {
                results.push(MatchedNode {
                    pathway: pathway.clone(),
                    node_kind: node.kind,
                    score: *score,
                    brief: node.digest.brief.clone(),
                    summary: Some(node.digest.summary.clone()),
                    content: None,
                    highlights: Vec::new(),
                });

                // Mark parent directory for exploration
                if let Some(parent) = pathway.parent() {
                    explored_dirs.insert(parent);
                }
            }
        }

        // Second pass: explore promising directories
        for dir_pathway in explored_dirs.iter().take(self.config.max_depth) {
            let children = self.storage.get_children(dir_pathway, 2).await?;

            for child in children {
                if child.is_directory || child.embedding.is_empty() {
                    continue;
                }

                let score = cosine_similarity(query_vector, &child.embedding);

                if score >= threshold {
                    // Check if already in results
                    let exists = results.iter().any(|r| r.pathway == child.pathway);
                    if !exists {
                        results.push(MatchedNode {
                            pathway: child.pathway,
                            node_kind: child.kind,
                            score,
                            brief: child.digest.brief,
                            summary: Some(child.digest.summary),
                            content: None,
                            highlights: Vec::new(),
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![0.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_normalized() {
        let a = vec![0.6, 0.8];
        let b = vec![0.8, 0.6];
        let sim = cosine_similarity(&a, &b);
        // cos(angle) = 0.6*0.8 + 0.8*0.6 = 0.96
        assert!((sim - 0.96).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_high_dimensional() {
        let a: Vec<f32> = (0..100).map(|i| (i as f32).sin()).collect();
        let b: Vec<f32> = (0..100).map(|i| (i as f32).sin()).collect();
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }
}
