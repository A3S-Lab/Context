//! Local file-based storage implementation

use async_trait::async_trait;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use crate::config::VectorIndexConfig;
use crate::core::{Namespace, Node};
use crate::error::Result;
use crate::pathway::Pathway;
use crate::{NodeInfo, StorageStats};

use super::{StorageBackend, VectorIndex};

pub struct LocalStorage {
    root_path: PathBuf,
    nodes: Arc<DashMap<String, Node>>,
    vector_index: Arc<VectorIndex>,
}

impl LocalStorage {
    pub async fn new(root_path: &Path, config: &VectorIndexConfig) -> Result<Self> {
        fs::create_dir_all(root_path).await?;

        let storage = Self {
            root_path: root_path.to_path_buf(),
            nodes: Arc::new(DashMap::new()),
            vector_index: Arc::new(VectorIndex::new(config)),
        };

        Ok(storage)
    }

    fn node_path(&self, pathway: &Pathway) -> PathBuf {
        let rel_path = pathway.to_relative().replace("://", "/");
        self.root_path.join(rel_path).with_extension("json")
    }

    async fn load_node(&self, pathway: &Pathway) -> Result<Node> {
        let path = self.node_path(pathway);

        if !path.exists() {
            return Err(crate::A3SError::NodeNotFound(pathway.to_string()));
        }

        let content = fs::read_to_string(&path).await?;
        let node: Node = serde_json::from_str(&content)?;

        Ok(node)
    }

    async fn save_node(&self, node: &Node) -> Result<()> {
        let path = self.node_path(&node.pathway);

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(node)?;
        fs::write(&path, content).await?;

        Ok(())
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn initialize(&self) -> Result<()> {
        // Load existing nodes
        // TODO: Implement node loading from disk

        Ok(())
    }

    async fn put(&self, node: &Node) -> Result<()> {
        // Save to disk
        self.save_node(node).await?;

        // Add to vector index if embedded
        if !node.embedding.is_empty() {
            self.vector_index
                .add(&node.pathway, &node.embedding)
                .await?;
        }

        // Cache in memory
        self.nodes.insert(node.pathway.to_string(), node.clone());

        Ok(())
    }

    async fn get(&self, pathway: &Pathway) -> Result<Node> {
        let key = pathway.to_string();

        // Check cache first
        if let Some(entry) = self.nodes.get(&key) {
            return Ok(entry.clone());
        }

        // Load from disk
        let node = self.load_node(pathway).await?;

        // Cache it
        self.nodes.insert(key, node.clone());

        Ok(node)
    }

    async fn exists(&self, pathway: &Pathway) -> Result<bool> {
        if self.nodes.contains_key(&pathway.to_string()) {
            return Ok(true);
        }

        Ok(self.node_path(pathway).exists())
    }

    async fn remove(&self, pathway: &Pathway, recursive: bool) -> Result<()> {
        let path = self.node_path(pathway);

        if recursive {
            // Remove directory and all children
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    fs::remove_dir_all(parent).await?;
                }
            }

            // Remove from cache
            let to_remove: Vec<String> = self
                .nodes
                .iter()
                .filter(|entry| pathway.is_prefix_of(&entry.value().pathway))
                .map(|entry| entry.key().clone())
                .collect();

            for k in to_remove {
                self.nodes.remove(&k);
            }
        } else {
            // Remove single file
            if path.exists() {
                fs::remove_file(&path).await?;
            }

            self.nodes.remove(&pathway.to_string());
        }

        // Remove from vector index
        self.vector_index.remove(pathway).await?;

        Ok(())
    }

    async fn list(&self, pathway: &Pathway) -> Result<Vec<NodeInfo>> {
        let mut results = Vec::new();

        // List from cache
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
        // All writes are immediate in this implementation
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
            self.save_node(&entry).await?;
            self.vector_index.add(pathway, &embedding).await?;
        }
        Ok(())
    }

    async fn update_digest(&self, pathway: &Pathway, digest: crate::digest::Digest) -> Result<()> {
        let key = pathway.to_string();
        if let Some(mut entry) = self.nodes.get_mut(&key) {
            entry.digest = digest;
            self.save_node(&entry).await?;
        }
        Ok(())
    }
}
