//! Storage backend abstraction and implementations

mod local;
mod memory;
mod vector_index;

pub use local::LocalStorage;
pub use memory::MemoryStorage;
pub use vector_index::VectorIndex;

use async_trait::async_trait;
use std::sync::Arc;

use crate::config::{StorageBackend as StorageBackendType, StorageConfig};
use crate::core::Node;
use crate::error::Result;
use crate::pathway::Pathway;
use crate::{NodeInfo, StorageStats};

/// Create a storage backend based on configuration
pub async fn create_backend(config: &StorageConfig) -> Result<Arc<dyn StorageBackend>> {
    match config.backend {
        StorageBackendType::Local => {
            let storage = LocalStorage::new(&config.path, &config.vector_index).await?;
            Ok(Arc::new(storage))
        }
        StorageBackendType::Memory => {
            let storage = MemoryStorage::new(&config.vector_index);
            Ok(Arc::new(storage))
        }
        StorageBackendType::Remote => {
            // TODO: Implement remote storage
            Err(crate::A3SError::Config(
                "Remote storage not yet implemented".to_string(),
            ))
        }
    }
}

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend
    async fn initialize(&self) -> Result<()>;

    /// Store a node
    async fn put(&self, node: &Node) -> Result<()>;

    /// Get a node by pathway
    async fn get(&self, pathway: &Pathway) -> Result<Node>;

    /// Check if a node exists
    async fn exists(&self, pathway: &Pathway) -> Result<bool>;

    /// Remove a node
    async fn remove(&self, pathway: &Pathway, recursive: bool) -> Result<()>;

    /// List nodes at a pathway
    async fn list(&self, pathway: &Pathway) -> Result<Vec<NodeInfo>>;

    /// Search by vector similarity
    async fn search_vector(
        &self,
        vector: &[f32],
        namespace: Option<crate::core::Namespace>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<(Pathway, f32)>>;

    /// Search by text pattern
    async fn search_text(
        &self,
        pattern: &str,
        pathway: &Pathway,
        case_insensitive: bool,
    ) -> Result<Vec<Pathway>>;

    /// Get storage statistics
    async fn stats(&self) -> Result<StorageStats>;

    /// Flush pending writes
    async fn flush(&self) -> Result<()>;

    /// Get all children of a pathway (recursive)
    async fn get_children(&self, pathway: &Pathway, max_depth: usize) -> Result<Vec<Node>>;

    /// Update node embedding
    async fn update_embedding(&self, pathway: &Pathway, embedding: Vec<f32>) -> Result<()>;

    /// Update node digest
    async fn update_digest(&self, pathway: &Pathway, digest: crate::digest::Digest) -> Result<()>;

    /// Batch insert nodes
    async fn put_batch(&self, nodes: &[Node]) -> Result<()> {
        for node in nodes {
            self.put(node).await?;
        }
        Ok(())
    }
}
