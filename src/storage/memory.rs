//! In-memory storage implementation (for testing)

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

use crate::config::VectorIndexConfig;
use crate::core::{Namespace, Node};
use crate::error::Result;
use crate::pathway::Pathway;
use crate::{NodeInfo, StorageStats};

use super::{StorageBackend, VectorIndex};

pub struct MemoryStorage {
    nodes: Arc<DashMap<String, Node>>,
    vector_index: Arc<VectorIndex>,
}

impl MemoryStorage {
    pub fn new(config: &VectorIndexConfig) -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
            vector_index: Arc::new(VectorIndex::new(config)),
        }
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn put(&self, node: &Node) -> Result<()> {
        let key = node.pathway.to_string();

        // Add to vector index if embedded
        if !node.embedding.is_empty() {
            self.vector_index
                .add(&node.pathway, &node.embedding)
                .await?;
        }

        self.nodes.insert(key, node.clone());
        Ok(())
    }

    async fn get(&self, pathway: &Pathway) -> Result<Node> {
        let key = pathway.to_string();
        self.nodes
            .get(&key)
            .map(|entry| entry.clone())
            .ok_or_else(|| crate::A3SError::NodeNotFound(pathway.to_string()))
    }

    async fn exists(&self, pathway: &Pathway) -> Result<bool> {
        Ok(self.nodes.contains_key(&pathway.to_string()))
    }

    async fn remove(&self, pathway: &Pathway, recursive: bool) -> Result<()> {
        let key = pathway.to_string();

        if recursive {
            // Remove all children
            let to_remove: Vec<String> = self
                .nodes
                .iter()
                .filter(|entry| {
                    let p = &entry.value().pathway;
                    pathway.is_prefix_of(p)
                })
                .map(|entry| entry.key().clone())
                .collect();

            for k in to_remove {
                self.nodes.remove(&k);
            }
        } else {
            self.nodes.remove(&key);
        }

        // Remove from vector index
        self.vector_index.remove(pathway).await?;

        Ok(())
    }

    async fn list(&self, pathway: &Pathway) -> Result<Vec<NodeInfo>> {
        let mut results = Vec::new();

        for entry in self.nodes.iter() {
            let node = entry.value();
            if let Some(parent) = node.pathway.parent() {
                if parent == *pathway {
                    results.push(NodeInfo {
                        pathway: node.pathway.clone(),
                        kind: node.kind,
                        is_directory: node.is_directory,
                        size: node.size(),
                        created_at: node.created_at,
                        updated_at: node.updated_at,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn search_vector(
        &self,
        vector: &[f32],
        namespace: Option<Namespace>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<(Pathway, f32)>> {
        self.vector_index
            .search(vector, namespace, limit, threshold)
            .await
    }

    async fn search_text(
        &self,
        pattern: &str,
        pathway: &Pathway,
        case_insensitive: bool,
    ) -> Result<Vec<Pathway>> {
        let pattern = if case_insensitive {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };

        let results: Vec<Pathway> = self
            .nodes
            .iter()
            .filter(|entry| {
                let node = entry.value();
                if !pathway.is_prefix_of(&node.pathway) {
                    return false;
                }

                let content = if case_insensitive {
                    node.content.to_lowercase()
                } else {
                    node.content.clone()
                };

                content.contains(&pattern)
            })
            .map(|entry| entry.value().pathway.clone())
            .collect();

        Ok(results)
    }

    async fn stats(&self) -> Result<StorageStats> {
        let mut stats = StorageStats::default();
        stats.total_nodes = self.nodes.len() as u64;

        for entry in self.nodes.iter() {
            let node = entry.value();
            if node.is_directory {
                stats.total_directories += 1;
            }
            stats.total_size_bytes += node.size();
        }

        Ok(stats)
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn get_children(&self, pathway: &Pathway, max_depth: usize) -> Result<Vec<Node>> {
        let results: Vec<Node> = self
            .nodes
            .iter()
            .filter(|entry| {
                let p = &entry.value().pathway;
                if !pathway.is_prefix_of(p) {
                    return false;
                }
                let depth = p.depth() - pathway.depth();
                depth > 0 && depth <= max_depth
            })
            .map(|entry| entry.value().clone())
            .collect();

        Ok(results)
    }

    async fn update_embedding(&self, pathway: &Pathway, embedding: Vec<f32>) -> Result<()> {
        let key = pathway.to_string();
        if let Some(mut entry) = self.nodes.get_mut(&key) {
            entry.embedding = embedding.clone();
            self.vector_index.add(pathway, &embedding).await?;
        }
        Ok(())
    }

    async fn update_digest(&self, pathway: &Pathway, digest: crate::digest::Digest) -> Result<()> {
        let key = pathway.to_string();
        if let Some(mut entry) = self.nodes.get_mut(&key) {
            entry.digest = digest;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Node, NodeKind};

    #[tokio::test]
    async fn test_memory_storage_put_and_get() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let storage = MemoryStorage::new(&config);

        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let node = Node::new(pathway.clone(), NodeKind::Document, "Test content".to_string());

        storage.put(&node).await.unwrap();

        let retrieved = storage.get(&pathway).await.unwrap();
        assert_eq!(retrieved.content, "Test content");
        assert_eq!(retrieved.pathway, pathway);
    }

    #[tokio::test]
    async fn test_memory_storage_exists() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let storage = MemoryStorage::new(&config);

        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        assert!(!storage.exists(&pathway).await.unwrap());

        let node = Node::new(pathway.clone(), NodeKind::Document, "Test".to_string());
        storage.put(&node).await.unwrap();

        assert!(storage.exists(&pathway).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_remove() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let storage = MemoryStorage::new(&config);

        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let node = Node::new(pathway.clone(), NodeKind::Document, "Test".to_string());
        storage.put(&node).await.unwrap();

        assert!(storage.exists(&pathway).await.unwrap());

        storage.remove(&pathway, false).await.unwrap();

        assert!(!storage.exists(&pathway).await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_list() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let storage = MemoryStorage::new(&config);

        let parent = Pathway::parse("a3s://knowledge/docs").unwrap();
        let child1 = Pathway::parse("a3s://knowledge/docs/file1").unwrap();
        let child2 = Pathway::parse("a3s://knowledge/docs/file2").unwrap();

        let node1 = Node::new(child1, NodeKind::Document, "Content 1".to_string());
        let node2 = Node::new(child2, NodeKind::Document, "Content 2".to_string());

        storage.put(&node1).await.unwrap();
        storage.put(&node2).await.unwrap();

        let list = storage.list(&parent).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_storage_stats() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let storage = MemoryStorage::new(&config);

        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let node = Node::new(pathway, NodeKind::Document, "Test content".to_string());
        storage.put(&node).await.unwrap();

        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.total_nodes, 1);
        assert!(stats.total_size_bytes > 0);
    }

    #[tokio::test]
    async fn test_memory_storage_update_embedding() {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        let storage = MemoryStorage::new(&config);

        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let node = Node::new(pathway.clone(), NodeKind::Document, "Test".to_string());
        storage.put(&node).await.unwrap();

        let embedding = vec![0.1, 0.2, 0.3];
        storage.update_embedding(&pathway, embedding.clone()).await.unwrap();

        let retrieved = storage.get(&pathway).await.unwrap();
        assert_eq!(retrieved.embedding, embedding);
    }
}
