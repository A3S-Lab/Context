//! Content ingestion and processing

use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;

use crate::config::Config;
use crate::core::{Node, NodeKind};
use crate::digest::DigestGenerator;
use crate::embedding::Embedder;
use crate::error::Result;
use crate::pathway::Pathway;
use crate::storage::StorageBackend;
use crate::IngestResult;

/// Content processor for ingesting files and directories
pub struct Processor {
    storage: Arc<dyn StorageBackend>,
    embedder: Arc<dyn Embedder>,
    digest_generator: DigestGenerator,
    config: Config,
}

impl Processor {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        embedder: Arc<dyn Embedder>,
        config: &Config,
    ) -> Self {
        let llm_client = if config.llm.auto_digest && config.llm.api_base.is_some() {
            Some(crate::digest::LLMClient::new(
                config.llm.api_base.clone().unwrap(),
                config.llm.api_key.clone().unwrap_or_default(),
                config.llm.model.clone().unwrap_or_default(),
            ))
        } else {
            None
        };

        Self {
            storage,
            embedder,
            digest_generator: DigestGenerator::new(llm_client),
            config: config.clone(),
        }
    }

    /// Process a source path and ingest into target pathway
    pub async fn process(&self, source: &str, target: &Pathway) -> Result<IngestResult> {
        let path = Path::new(source);

        if !path.exists() {
            return Err(crate::A3SError::Ingest(format!(
                "Source path does not exist: {}",
                source
            )));
        }

        let mut nodes_created = 0;
        let mut nodes_updated = 0;
        let mut errors = Vec::new();

        if path.is_file() {
            match self.process_file(path, target).await {
                Ok(created) => {
                    if created {
                        nodes_created += 1;
                    } else {
                        nodes_updated += 1;
                    }
                }
                Err(e) => errors.push(format!("{}: {}", source, e)),
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| !self.should_ignore(e.path()))
            {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        errors.push(format!("Walk error: {}", e));
                        continue;
                    }
                };

                if entry.file_type().is_file() {
                    let rel_path = entry
                        .path()
                        .strip_prefix(path)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    let file_pathway = target.join(&rel_path);

                    match self.process_file(entry.path(), &file_pathway).await {
                        Ok(created) => {
                            if created {
                                nodes_created += 1;
                            } else {
                                nodes_updated += 1;
                            }
                        }
                        Err(e) => errors.push(format!("{}: {}", rel_path, e)),
                    }
                }
            }
        }

        Ok(IngestResult {
            pathway: target.clone(),
            nodes_created,
            nodes_updated,
            errors,
        })
    }

    async fn process_file(&self, path: &Path, pathway: &Pathway) -> Result<bool> {
        // Check file size
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > self.config.ingest.max_file_size {
            return Err(crate::A3SError::Ingest(format!(
                "File too large: {} bytes",
                metadata.len()
            )));
        }

        // Read content
        let content = std::fs::read_to_string(path)?;

        // Determine node kind
        let kind = self.detect_kind(path);

        // Check if node exists
        let exists = self.storage.exists(pathway).await?;

        // Create or update node
        let mut node = if exists {
            let mut existing = self.storage.get(pathway).await?;
            existing.update_content(content);
            existing
        } else {
            Node::new(pathway.clone(), kind, content)
        };

        // Generate digest
        if self.config.llm.auto_digest {
            node.digest = self
                .digest_generator
                .generate(&node.content, node.kind)
                .await?;
        }

        // Generate embedding
        let embedding = self.embedder.embed(&node.content).await?;
        node.embedding = embedding;

        // Store node
        self.storage.put(&node).await?;

        Ok(!exists)
    }

    fn detect_kind(&self, path: &Path) -> NodeKind {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match ext {
            "md" => NodeKind::Markdown,
            "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" => NodeKind::Code,
            _ => NodeKind::Document,
        }
    }

    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.ingest.ignore_patterns {
            if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }
}
