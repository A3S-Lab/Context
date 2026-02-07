//! Simple vector index implementation

use dashmap::DashMap;
use ordered_float::OrderedFloat;
use std::collections::BinaryHeap;
use std::sync::Arc;

use crate::config::VectorIndexConfig;
use crate::core::Namespace;
use crate::error::Result;
use crate::pathway::Pathway;

/// Simple in-memory vector index
pub struct VectorIndex {
    vectors: Arc<DashMap<String, Vec<f32>>>,
    #[allow(dead_code)]
    config: VectorIndexConfig,
}

impl VectorIndex {
    pub fn new(config: &VectorIndexConfig) -> Self {
        Self {
            vectors: Arc::new(DashMap::new()),
            config: config.clone(),
        }
    }

    pub async fn add(&self, pathway: &Pathway, vector: &[f32]) -> Result<()> {
        self.vectors.insert(pathway.to_string(), vector.to_vec());
        Ok(())
    }

    pub async fn remove(&self, pathway: &Pathway) -> Result<()> {
        self.vectors.remove(&pathway.to_string());
        Ok(())
    }

    pub async fn search(
        &self,
        query: &[f32],
        namespace: Option<Namespace>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<(Pathway, f32)>> {
        let mut heap = BinaryHeap::new();

        for entry in self.vectors.iter() {
            let pathway = Pathway::parse(entry.key())?;

            // Filter by namespace if specified
            if let Some(ns) = namespace {
                if pathway.namespace() != ns {
                    continue;
                }
            }

            let score = cosine_similarity(query, entry.value());

            if score >= threshold {
                heap.push((OrderedFloat(score), pathway));
            }
        }

        let mut results = Vec::new();
        for _ in 0..limit {
            if let Some((score, pathway)) = heap.pop() {
                results.push((pathway, score.0));
            } else {
                break;
            }
        }

        Ok(results)
    }

    pub fn size(&self) -> usize {
        self.vectors.len()
    }
}

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

    #[tokio::test]
    async fn test_vector_index_add_and_search() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let index = VectorIndex::new(&config);

        let p1 = Pathway::parse("a3s://knowledge/doc1").unwrap();
        let v1 = vec![1.0, 0.0, 0.0];
        index.add(&p1, &v1).await.unwrap();

        let p2 = Pathway::parse("a3s://knowledge/doc2").unwrap();
        let v2 = vec![0.0, 1.0, 0.0];
        index.add(&p2, &v2).await.unwrap();

        assert_eq!(index.size(), 2);

        // Search for similar to v1
        let query = vec![0.9, 0.1, 0.0];
        let results = index.search(&query, None, 10, 0.5).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].0, p1);
    }

    #[tokio::test]
    async fn test_vector_index_remove() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let index = VectorIndex::new(&config);

        let p1 = Pathway::parse("a3s://knowledge/doc1").unwrap();
        let v1 = vec![1.0, 0.0, 0.0];
        index.add(&p1, &v1).await.unwrap();

        assert_eq!(index.size(), 1);

        index.remove(&p1).await.unwrap();
        assert_eq!(index.size(), 0);
    }

    #[tokio::test]
    async fn test_vector_index_namespace_filter() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let index = VectorIndex::new(&config);

        let p1 = Pathway::parse("a3s://knowledge/doc1").unwrap();
        let v1 = vec![1.0, 0.0, 0.0];
        index.add(&p1, &v1).await.unwrap();

        let p2 = Pathway::parse("a3s://memory/mem1").unwrap();
        let v2 = vec![1.0, 0.0, 0.0];
        index.add(&p2, &v2).await.unwrap();

        // Search only in knowledge namespace
        let query = vec![1.0, 0.0, 0.0];
        let results = index
            .search(&query, Some(Namespace::Knowledge), 10, 0.5)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, p1);
    }

    #[tokio::test]
    async fn test_vector_index_threshold() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let index = VectorIndex::new(&config);

        let p1 = Pathway::parse("a3s://knowledge/doc1").unwrap();
        let v1 = vec![1.0, 0.0, 0.0];
        index.add(&p1, &v1).await.unwrap();

        // High threshold should filter out results
        let query = vec![0.5, 0.5, 0.0];
        let results = index.search(&query, None, 10, 0.9).await.unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }
}
