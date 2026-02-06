//! Core data structures for A3S Context

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::digest::Digest;
use crate::pathway::Pathway;

/// Type of namespace for organizing context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Namespace {
    /// Knowledge base (documents, code, etc.)
    Knowledge,
    /// Agent and user memories
    Memory,
    /// Agent capabilities and tools
    Capability,
    /// Active sessions
    Session,
}

impl Namespace {
    pub fn as_str(&self) -> &'static str {
        match self {
            Namespace::Knowledge => "knowledge",
            Namespace::Memory => "memory",
            Namespace::Capability => "capability",
            Namespace::Session => "session",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "knowledge" => Some(Namespace::Knowledge),
            "memory" => Some(Namespace::Memory),
            "capability" => Some(Namespace::Capability),
            "session" => Some(Namespace::Session),
            _ => None,
        }
    }
}

/// Kind of node content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// Directory node (container)
    Directory,
    /// Text document
    Document,
    /// Source code
    Code,
    /// Markdown content
    Markdown,
    /// Memory entry
    Memory,
    /// Capability definition
    Capability,
    /// Session message
    Message,
    /// Generic data
    Data,
}

/// A node in the A3S context tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier
    pub id: Uuid,

    /// Full pathway to this node
    pub pathway: Pathway,

    /// Type of node
    pub kind: NodeKind,

    /// Whether this is a directory
    pub is_directory: bool,

    /// Multi-level digest
    pub digest: Digest,

    /// Full content (may be empty for directories)
    pub content: String,

    /// Embedding vector
    pub embedding: Vec<f32>,

    /// Metadata
    pub metadata: Metadata,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Related pathways
    pub relations: Vec<Relation>,
}

impl Node {
    /// Create a new node
    pub fn new(pathway: Pathway, kind: NodeKind, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            pathway,
            kind,
            is_directory: false,
            digest: Digest::default(),
            content,
            embedding: Vec::new(),
            metadata: Metadata::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            relations: Vec::new(),
        }
    }

    /// Create a directory node
    pub fn directory(pathway: Pathway) -> Self {
        Self {
            id: Uuid::new_v4(),
            pathway,
            kind: NodeKind::Directory,
            is_directory: true,
            digest: Digest::default(),
            content: String::new(),
            embedding: Vec::new(),
            metadata: Metadata::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            relations: Vec::new(),
        }
    }

    /// Get the namespace of this node
    pub fn namespace(&self) -> Namespace {
        self.pathway.namespace()
    }

    /// Get the size in bytes
    pub fn size(&self) -> u64 {
        self.content.len() as u64
    }

    /// Check if this node has been embedded
    pub fn is_embedded(&self) -> bool {
        !self.embedding.is_empty()
    }

    /// Update the content and reset digest
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
        self.digest = Digest::default();
    }

    /// Add a relation to another node
    pub fn add_relation(&mut self, target: Pathway, kind: RelationKind, reason: String) {
        self.relations.push(Relation {
            target,
            kind,
            reason,
            created_at: Utc::now(),
        });
    }
}

/// Metadata associated with a node
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Custom key-value pairs
    pub custom: HashMap<String, serde_json::Value>,

    /// Source information
    pub source: Option<SourceInfo>,

    /// Access count
    pub access_count: u64,

    /// Last accessed time
    pub last_accessed: Option<DateTime<Utc>>,

    /// Tags
    pub tags: Vec<String>,
}

/// Source information for ingested content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Original path or URL
    pub origin: String,

    /// Content type
    pub content_type: Option<String>,

    /// File size
    pub size: u64,

    /// Hash of original content
    pub hash: String,
}

/// Relation between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Target pathway
    pub target: Pathway,

    /// Type of relation
    pub kind: RelationKind,

    /// Reason for the relation
    pub reason: String,

    /// When the relation was created
    pub created_at: DateTime<Utc>,
}

/// Type of relation between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationKind {
    /// References another node
    References,

    /// Derived from another node
    DerivedFrom,

    /// Related to another node
    RelatedTo,

    /// Depends on another node
    DependsOn,

    /// Custom relation
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_as_str() {
        assert_eq!(Namespace::Knowledge.as_str(), "knowledge");
        assert_eq!(Namespace::Memory.as_str(), "memory");
        assert_eq!(Namespace::Capability.as_str(), "capability");
        assert_eq!(Namespace::Session.as_str(), "session");
    }

    #[test]
    fn test_namespace_from_str() {
        assert_eq!(Namespace::parse("knowledge"), Some(Namespace::Knowledge));
        assert_eq!(Namespace::parse("memory"), Some(Namespace::Memory));
        assert_eq!(
            Namespace::parse("capability"),
            Some(Namespace::Capability)
        );
        assert_eq!(Namespace::parse("session"), Some(Namespace::Session));
        assert_eq!(Namespace::parse("invalid"), None);
    }

    #[test]
    fn test_node_new() {
        let pathway = Pathway::parse("a3s://knowledge/docs/test").unwrap();
        let node = Node::new(
            pathway.clone(),
            NodeKind::Document,
            "Test content".to_string(),
        );

        assert_eq!(node.pathway, pathway);
        assert_eq!(node.kind, NodeKind::Document);
        assert_eq!(node.content, "Test content");
        assert!(!node.is_directory);
        assert!(!node.is_embedded());
    }

    #[test]
    fn test_node_directory() {
        let pathway = Pathway::parse("a3s://knowledge/docs").unwrap();
        let node = Node::directory(pathway.clone());

        assert_eq!(node.pathway, pathway);
        assert_eq!(node.kind, NodeKind::Directory);
        assert!(node.is_directory);
        assert!(node.content.is_empty());
    }

    #[test]
    fn test_node_namespace() {
        let pathway = Pathway::parse("a3s://memory/user/prefs").unwrap();
        let node = Node::new(pathway, NodeKind::Memory, "prefs".to_string());

        assert_eq!(node.namespace(), Namespace::Memory);
    }

    #[test]
    fn test_node_size() {
        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let node = Node::new(pathway, NodeKind::Document, "Hello".to_string());

        assert_eq!(node.size(), 5);
    }

    #[test]
    fn test_node_is_embedded() {
        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let mut node = Node::new(pathway, NodeKind::Document, "Test".to_string());

        assert!(!node.is_embedded());

        node.embedding = vec![0.1, 0.2, 0.3];
        assert!(node.is_embedded());
    }

    #[test]
    fn test_node_update_content() {
        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let mut node = Node::new(pathway, NodeKind::Document, "Old content".to_string());
        let original_updated = node.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        node.update_content("New content".to_string());

        assert_eq!(node.content, "New content");
        assert!(node.updated_at > original_updated);
    }

    #[test]
    fn test_node_add_relation() {
        let pathway = Pathway::parse("a3s://knowledge/test").unwrap();
        let target = Pathway::parse("a3s://knowledge/related").unwrap();
        let mut node = Node::new(pathway, NodeKind::Document, "Test".to_string());

        assert!(node.relations.is_empty());

        node.add_relation(
            target.clone(),
            RelationKind::References,
            "Test relation".to_string(),
        );

        assert_eq!(node.relations.len(), 1);
        assert_eq!(node.relations[0].target, target);
        assert_eq!(node.relations[0].kind, RelationKind::References);
        assert_eq!(node.relations[0].reason, "Test relation");
    }

    #[test]
    fn test_metadata_default() {
        let metadata = Metadata::default();

        assert!(metadata.custom.is_empty());
        assert!(metadata.source.is_none());
        assert_eq!(metadata.access_count, 0);
        assert!(metadata.last_accessed.is_none());
        assert!(metadata.tags.is_empty());
    }
}
