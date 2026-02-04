//! # A3S Context
//!
//! Autonomous Agent Adaptive Storage - A hierarchical context management system for AI agents.
//!
//! A3S provides a unified way to manage different types of context (knowledge, memory, capabilities)
//! with automatic summarization, semantic search, and intelligent retrieval.
//!
//! ## Core Concepts
//!
//! - **Namespace**: Logical grouping of context items (e.g., `knowledge`, `memory`, `capability`)
//! - **Node**: A single context item with content and metadata
//! - **Digest**: Multi-level summaries (Brief/Summary/Full) for efficient retrieval
//! - **Pathway**: URI-like addressing scheme (`a3s://namespace/path/to/node`)
//!
//! ## Example
//!
//! ```rust,no_run
//! use a3s_context::{A3SClient, Config};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = A3SClient::new(Config::default()).await?;
//!
//!     // Add knowledge
//!     client.ingest("./docs", "a3s://knowledge/docs").await?;
//!
//!     // Search
//!     let results = client.query("How does authentication work?").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod core;
pub mod digest;
pub mod embedding;
pub mod error;
pub mod ingest;
pub mod pathway;
pub mod retrieval;
pub mod session;
pub mod storage;
pub mod config;

pub use crate::config::Config;
pub use crate::core::{Node, NodeKind, Namespace};
pub use crate::error::{A3SError, Result};
pub use crate::pathway::Pathway;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Main client for interacting with A3S Context
pub struct A3SClient {
    config: Config,
    storage: Arc<dyn storage::StorageBackend>,
    embedder: Arc<dyn embedding::Embedder>,
    state: Arc<RwLock<ClientState>>,
}

struct ClientState {
    initialized: bool,
    active_sessions: dashmap::DashMap<String, session::Session>,
}

impl A3SClient {
    /// Create a new A3S client with the given configuration
    pub async fn new(config: Config) -> Result<Self> {
        let storage = storage::create_backend(&config.storage).await?;
        let embedder = embedding::create_embedder(&config.embedding).await?;

        let state = Arc::new(RwLock::new(ClientState {
            initialized: false,
            active_sessions: dashmap::DashMap::new(),
        }));

        let client = Self {
            config,
            storage,
            embedder,
            state,
        };

        client.initialize().await?;

        Ok(client)
    }

    /// Initialize the storage backend
    async fn initialize(&self) -> Result<()> {
        self.storage.initialize().await?;

        let mut state = self.state.write().await;
        state.initialized = true;

        tracing::info!("A3S Context initialized successfully");
        Ok(())
    }

    /// Ingest content from a source path into the specified pathway
    pub async fn ingest<P: AsRef<str>, T: AsRef<str>>(
        &self,
        source: P,
        target: T,
    ) -> Result<IngestResult> {
        let pathway = Pathway::parse(target.as_ref())?;
        let processor = ingest::Processor::new(
            self.storage.clone(),
            self.embedder.clone(),
            &self.config,
        );

        processor.process(source.as_ref(), &pathway).await
    }

    /// Query the context store with natural language
    pub async fn query(&self, query: &str) -> Result<QueryResult> {
        let retriever = retrieval::Retriever::new(
            self.storage.clone(),
            self.embedder.clone(),
            &self.config.retrieval,
        );

        retriever.search(query, None).await
    }

    /// Query with additional options
    pub async fn query_with_options(
        &self,
        query: &str,
        options: QueryOptions,
    ) -> Result<QueryResult> {
        let retriever = retrieval::Retriever::new(
            self.storage.clone(),
            self.embedder.clone(),
            &self.config.retrieval,
        );

        retriever.search(query, Some(options)).await
    }

    /// List nodes at a pathway
    pub async fn list<P: AsRef<str>>(&self, pathway: P) -> Result<Vec<NodeInfo>> {
        let pathway = Pathway::parse(pathway.as_ref())?;
        self.storage.list(&pathway).await
    }

    /// Read a node's content
    pub async fn read<P: AsRef<str>>(&self, pathway: P) -> Result<Node> {
        let pathway = Pathway::parse(pathway.as_ref())?;
        self.storage.get(&pathway).await
    }

    /// Read a node's brief digest (smallest summary)
    pub async fn brief<P: AsRef<str>>(&self, pathway: P) -> Result<String> {
        let pathway = Pathway::parse(pathway.as_ref())?;
        let node = self.storage.get(&pathway).await?;
        Ok(node.digest.brief)
    }

    /// Read a node's summary digest (medium summary)
    pub async fn summary<P: AsRef<str>>(&self, pathway: P) -> Result<String> {
        let pathway = Pathway::parse(pathway.as_ref())?;
        let node = self.storage.get(&pathway).await?;
        Ok(node.digest.summary)
    }

    /// Remove a node or directory
    pub async fn remove<P: AsRef<str>>(&self, pathway: P, recursive: bool) -> Result<()> {
        let pathway = Pathway::parse(pathway.as_ref())?;
        self.storage.remove(&pathway, recursive).await
    }

    /// Create a new session for conversation tracking
    pub async fn session(&self, id: Option<&str>) -> Result<session::Session> {
        let session = session::Session::new(
            id,
            self.storage.clone(),
            self.embedder.clone(),
            &self.config,
        ).await?;

        let state = self.state.read().await;
        state.active_sessions.insert(session.id().to_string(), session.clone());

        Ok(session)
    }

    /// Get storage statistics
    pub async fn stats(&self) -> Result<StorageStats> {
        self.storage.stats().await
    }

    /// Shutdown the client gracefully
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down A3S Context");
        self.storage.flush().await?;
        Ok(())
    }
}

/// Result of an ingest operation
#[derive(Debug, Clone)]
pub struct IngestResult {
    pub pathway: Pathway,
    pub nodes_created: usize,
    pub nodes_updated: usize,
    pub errors: Vec<String>,
}

/// Options for query operations
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub namespace: Option<Namespace>,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub include_content: bool,
    pub pathway_filter: Option<String>,
}

/// Result of a query operation
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub matches: Vec<MatchedNode>,
    pub total_searched: usize,
    pub query_embedding_time_ms: u64,
    pub search_time_ms: u64,
}

/// A matched node from a query
#[derive(Debug, Clone)]
pub struct MatchedNode {
    pub pathway: Pathway,
    pub node_kind: NodeKind,
    pub score: f32,
    pub brief: String,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub highlights: Vec<String>,
}

/// Basic node information for listing
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub pathway: Pathway,
    pub kind: NodeKind,
    pub is_directory: bool,
    pub size: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_nodes: u64,
    pub total_directories: u64,
    pub total_size_bytes: u64,
    pub namespaces: Vec<NamespaceStats>,
}

/// Statistics for a single namespace
#[derive(Debug, Clone)]
pub struct NamespaceStats {
    pub namespace: Namespace,
    pub node_count: u64,
    pub size_bytes: u64,
}
