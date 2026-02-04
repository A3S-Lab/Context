//! Configuration for A3S Context

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for A3S Context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,

    /// Embedding configuration
    #[serde(default)]
    pub embedding: EmbeddingConfig,

    /// LLM configuration for digest generation
    #[serde(default)]
    pub llm: LLMConfig,

    /// Retrieval configuration
    #[serde(default)]
    pub retrieval: RetrievalConfig,

    /// Ingest configuration
    #[serde(default)]
    pub ingest: IngestConfig,

    /// Logging level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            embedding: EmbeddingConfig::default(),
            llm: LLMConfig::default(),
            retrieval: RetrievalConfig::default(),
            ingest: IngestConfig::default(),
            log_level: default_log_level(),
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        let config: Config = if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| crate::A3SError::Config(e.to_string()))?
        } else if path.ends_with(".toml") {
            toml::from_str(&content)
                .map_err(|e| crate::A3SError::Config(e.to_string()))?
        } else {
            serde_json::from_str(&content)?
        };

        Ok(config)
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Storage
        if let Ok(path) = std::env::var("A3S_STORAGE_PATH") {
            config.storage.path = PathBuf::from(path);
        }

        // Embedding
        if let Ok(api_base) = std::env::var("A3S_EMBEDDING_API_BASE") {
            config.embedding.api_base = Some(api_base);
        }
        if let Ok(api_key) = std::env::var("A3S_EMBEDDING_API_KEY") {
            config.embedding.api_key = Some(api_key);
        }
        if let Ok(model) = std::env::var("A3S_EMBEDDING_MODEL") {
            config.embedding.model = model;
        }

        // LLM
        if let Ok(api_base) = std::env::var("A3S_LLM_API_BASE") {
            config.llm.api_base = Some(api_base);
        }
        if let Ok(api_key) = std::env::var("A3S_LLM_API_KEY") {
            config.llm.api_key = Some(api_key);
        }
        if let Ok(model) = std::env::var("A3S_LLM_MODEL") {
            config.llm.model = Some(model);
        }

        // Log level
        if let Ok(level) = std::env::var("A3S_LOG_LEVEL") {
            config.log_level = level;
        }

        config
    }

    /// Merge with another config (other takes precedence)
    pub fn merge(mut self, other: Config) -> Self {
        if other.storage.path != PathBuf::from("./a3s_data") {
            self.storage = other.storage;
        }
        if other.embedding.api_base.is_some() {
            self.embedding = other.embedding;
        }
        if other.llm.api_base.is_some() {
            self.llm = other.llm;
        }
        self.retrieval = other.retrieval;
        self.ingest = other.ingest;
        if other.log_level != "info" {
            self.log_level = other.log_level;
        }
        self
    }
}

/// Storage backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Backend type
    #[serde(default = "default_storage_backend")]
    pub backend: StorageBackend,

    /// Local storage path
    #[serde(default = "default_storage_path")]
    pub path: PathBuf,

    /// Remote storage URL (for remote backend)
    pub url: Option<String>,

    /// Vector index configuration
    #[serde(default)]
    pub vector_index: VectorIndexConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: default_storage_backend(),
            path: default_storage_path(),
            url: None,
            vector_index: VectorIndexConfig::default(),
        }
    }
}

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// Local file-based storage
    Local,
    /// Remote storage service
    Remote,
    /// In-memory storage (for testing)
    Memory,
}

/// Vector index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexConfig {
    /// Index type
    #[serde(default = "default_index_type")]
    pub index_type: String,

    /// Number of neighbors for HNSW
    #[serde(default = "default_hnsw_m")]
    pub hnsw_m: usize,

    /// Construction parameter for HNSW
    #[serde(default = "default_hnsw_ef_construction")]
    pub hnsw_ef_construction: usize,
}

impl Default for VectorIndexConfig {
    fn default() -> Self {
        Self {
            index_type: default_index_type(),
            hnsw_m: default_hnsw_m(),
            hnsw_ef_construction: default_hnsw_ef_construction(),
        }
    }
}

/// Embedding model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider type
    #[serde(default = "default_embedding_provider")]
    pub provider: String,

    /// API base URL
    pub api_base: Option<String>,

    /// API key
    pub api_key: Option<String>,

    /// Model name
    #[serde(default = "default_embedding_model")]
    pub model: String,

    /// Embedding dimension
    #[serde(default = "default_embedding_dimension")]
    pub dimension: usize,

    /// Batch size for embedding
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_embedding_provider(),
            api_base: None,
            api_key: None,
            model: default_embedding_model(),
            dimension: default_embedding_dimension(),
            batch_size: default_batch_size(),
        }
    }
}

/// LLM configuration for digest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// Provider type
    #[serde(default = "default_llm_provider")]
    pub provider: String,

    /// API base URL
    pub api_base: Option<String>,

    /// API key
    pub api_key: Option<String>,

    /// Model name
    pub model: Option<String>,

    /// Temperature
    #[serde(default)]
    pub temperature: f32,

    /// Whether to auto-generate digests
    #[serde(default = "default_auto_digest")]
    pub auto_digest: bool,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            api_base: None,
            api_key: None,
            model: None,
            temperature: 0.0,
            auto_digest: default_auto_digest(),
        }
    }
}

/// Retrieval configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Default result limit
    #[serde(default = "default_limit")]
    pub default_limit: usize,

    /// Score threshold
    #[serde(default = "default_threshold")]
    pub score_threshold: f32,

    /// Enable hierarchical retrieval
    #[serde(default = "default_hierarchical")]
    pub hierarchical: bool,

    /// Maximum depth for hierarchical search
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Enable reranking
    #[serde(default)]
    pub rerank: bool,

    /// Rerank model
    pub rerank_model: Option<String>,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            default_limit: default_limit(),
            score_threshold: default_threshold(),
            hierarchical: default_hierarchical(),
            max_depth: default_max_depth(),
            rerank: false,
            rerank_model: None,
        }
    }
}

/// Ingest configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Supported file extensions
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    /// Maximum file size in bytes
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Chunk size for large documents
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,

    /// Chunk overlap
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,

    /// Ignore patterns
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            extensions: default_extensions(),
            max_file_size: default_max_file_size(),
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
            ignore_patterns: default_ignore_patterns(),
        }
    }
}

// Default value functions
fn default_log_level() -> String {
    "info".to_string()
}

fn default_storage_backend() -> StorageBackend {
    StorageBackend::Local
}

fn default_storage_path() -> PathBuf {
    PathBuf::from("./a3s_data")
}

fn default_index_type() -> String {
    "hnsw".to_string()
}

fn default_hnsw_m() -> usize {
    16
}

fn default_hnsw_ef_construction() -> usize {
    200
}

fn default_embedding_provider() -> String {
    "openai".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

fn default_embedding_dimension() -> usize {
    1536
}

fn default_batch_size() -> usize {
    32
}

fn default_llm_provider() -> String {
    "openai".to_string()
}

fn default_auto_digest() -> bool {
    true
}

fn default_limit() -> usize {
    10
}

fn default_threshold() -> f32 {
    0.5
}

fn default_hierarchical() -> bool {
    true
}

fn default_max_depth() -> usize {
    3
}

fn default_extensions() -> Vec<String> {
    vec![
        "md".to_string(),
        "txt".to_string(),
        "rs".to_string(),
        "py".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "go".to_string(),
        "java".to_string(),
        "c".to_string(),
        "cpp".to_string(),
        "h".to_string(),
        "json".to_string(),
        "yaml".to_string(),
        "toml".to_string(),
    ]
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_chunk_size() -> usize {
    1000
}

fn default_chunk_overlap() -> usize {
    200
}

fn default_ignore_patterns() -> Vec<String> {
    vec![
        ".git".to_string(),
        "node_modules".to_string(),
        "target".to_string(),
        "__pycache__".to_string(),
        ".venv".to_string(),
        "*.pyc".to_string(),
        "*.pyo".to_string(),
        ".DS_Store".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.storage.backend, StorageBackend::Local);
        assert_eq!(config.embedding.provider, "openai");
        assert!(config.llm.auto_digest);
        assert_eq!(config.retrieval.default_limit, 10);
    }

    #[test]
    fn test_storage_backend() {
        assert_eq!(StorageBackend::Local, StorageBackend::Local);
        assert_ne!(StorageBackend::Local, StorageBackend::Memory);
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.backend, StorageBackend::Local);
        assert_eq!(config.path, std::path::PathBuf::from("./a3s_data"));
        assert!(config.url.is_none());
    }

    #[test]
    fn test_vector_index_config_default() {
        let config = VectorIndexConfig::default();
        assert_eq!(config.index_type, "hnsw");
        assert_eq!(config.hnsw_m, 16);
        assert_eq!(config.hnsw_ef_construction, 200);
    }

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.dimension, 1536);
        assert_eq!(config.batch_size, 32);
    }

    #[test]
    fn test_llm_config_default() {
        let config = LLMConfig::default();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.temperature, 0.0);
        assert!(config.auto_digest);
    }

    #[test]
    fn test_retrieval_config_default() {
        let config = RetrievalConfig::default();
        assert_eq!(config.default_limit, 10);
        assert_eq!(config.score_threshold, 0.5);
        assert!(config.hierarchical);
        assert_eq!(config.max_depth, 3);
        assert!(!config.rerank);
    }

    #[test]
    fn test_ingest_config_default() {
        let config = IngestConfig::default();
        assert!(!config.extensions.is_empty());
        assert!(config.extensions.contains(&"rs".to_string()));
        assert!(config.extensions.contains(&"md".to_string()));
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.chunk_size, 1000);
        assert_eq!(config.chunk_overlap, 200);
        assert!(!config.ignore_patterns.is_empty());
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("A3S_LOG_LEVEL", "debug");
        std::env::set_var("A3S_STORAGE_PATH", "/tmp/test");

        let config = Config::from_env();

        assert_eq!(config.log_level, "debug");
        assert_eq!(config.storage.path, std::path::PathBuf::from("/tmp/test"));

        std::env::remove_var("A3S_LOG_LEVEL");
        std::env::remove_var("A3S_STORAGE_PATH");
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = Config::default();
        config1.log_level = "info".to_string();

        let mut config2 = Config::default();
        config2.log_level = "debug".to_string();

        let merged = config1.merge(config2);
        assert_eq!(merged.log_level, "debug");
    }

    #[test]
    fn test_default_functions() {
        assert_eq!(default_log_level(), "info");
        assert_eq!(default_storage_backend(), StorageBackend::Local);
        assert_eq!(default_index_type(), "hnsw");
        assert_eq!(default_hnsw_m(), 16);
        assert_eq!(default_embedding_provider(), "openai");
        assert_eq!(default_embedding_model(), "text-embedding-3-small");
        assert_eq!(default_embedding_dimension(), 1536);
        assert_eq!(default_batch_size(), 32);
        assert!(default_auto_digest());
        assert_eq!(default_limit(), 10);
        assert_eq!(default_threshold(), 0.5);
        assert!(default_hierarchical());
        assert_eq!(default_max_depth(), 3);
        assert_eq!(default_max_file_size(), 10 * 1024 * 1024);
        assert_eq!(default_chunk_size(), 1000);
        assert_eq!(default_chunk_overlap(), 200);
    }
}
