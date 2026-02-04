//! Pathway - URI-like addressing for A3S nodes
//!
//! Format: `a3s://namespace/path/to/node`
//!
//! Examples:
//! - `a3s://knowledge/docs/api`
//! - `a3s://memory/user/preferences`
//! - `a3s://capability/tools/search`

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::core::Namespace;
use crate::error::{A3SError, Result};

/// A pathway represents a unique address to a node in A3S
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Pathway {
    namespace: Namespace,
    segments: Vec<String>,
}

impl Pathway {
    /// Protocol prefix
    pub const PROTOCOL: &'static str = "a3s://";

    /// Create a new pathway
    pub fn new(namespace: Namespace, segments: Vec<String>) -> Self {
        Self { namespace, segments }
    }

    /// Parse a pathway from a string
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        // Handle protocol prefix
        let path_str = if s.starts_with(Self::PROTOCOL) {
            &s[Self::PROTOCOL.len()..]
        } else if s.starts_with('/') {
            &s[1..]
        } else {
            s
        };

        if path_str.is_empty() {
            return Err(A3SError::InvalidPathway("Empty pathway".to_string()));
        }

        let parts: Vec<&str> = path_str.split('/').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return Err(A3SError::InvalidPathway("No namespace specified".to_string()));
        }

        let namespace = Namespace::from_str(parts[0]).ok_or_else(|| {
            A3SError::InvalidPathway(format!("Invalid namespace: {}", parts[0]))
        })?;

        let segments: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        // Validate segments
        for seg in &segments {
            if seg.is_empty() || seg.contains('\0') {
                return Err(A3SError::InvalidPathway(format!(
                    "Invalid segment: {:?}",
                    seg
                )));
            }
        }

        Ok(Self { namespace, segments })
    }

    /// Get the namespace
    pub fn namespace(&self) -> Namespace {
        self.namespace
    }

    /// Get the path segments
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Get the name (last segment)
    pub fn name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Get the parent pathway
    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            None
        } else {
            Some(Self {
                namespace: self.namespace,
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }

    /// Join a child segment
    pub fn join(&self, segment: &str) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.to_string());
        Self {
            namespace: self.namespace,
            segments,
        }
    }

    /// Check if this pathway is a prefix of another
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        if self.namespace != other.namespace {
            return false;
        }
        if self.segments.len() > other.segments.len() {
            return false;
        }
        self.segments
            .iter()
            .zip(other.segments.iter())
            .all(|(a, b)| a == b)
    }

    /// Check if this is a root namespace pathway
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get depth (number of segments)
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Convert to a relative path string
    pub fn to_relative(&self) -> String {
        if self.segments.is_empty() {
            self.namespace.as_str().to_string()
        } else {
            format!("{}/{}", self.namespace.as_str(), self.segments.join("/"))
        }
    }

    /// Create a root pathway for a namespace
    pub fn root(namespace: Namespace) -> Self {
        Self {
            namespace,
            segments: Vec::new(),
        }
    }

    /// Create a knowledge pathway
    pub fn knowledge(path: &str) -> Result<Self> {
        Self::parse(&format!("a3s://knowledge/{}", path))
    }

    /// Create a memory pathway
    pub fn memory(path: &str) -> Result<Self> {
        Self::parse(&format!("a3s://memory/{}", path))
    }

    /// Create a capability pathway
    pub fn capability(path: &str) -> Result<Self> {
        Self::parse(&format!("a3s://capability/{}", path))
    }
}

impl fmt::Display for Pathway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PROTOCOL, self.to_relative())
    }
}

impl TryFrom<&str> for Pathway {
    type Error = A3SError;

    fn try_from(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Pathway {
    type Error = A3SError;

    fn try_from(s: String) -> Result<Self> {
        Self::parse(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pathway() {
        let p = Pathway::parse("a3s://knowledge/docs/api").unwrap();
        assert_eq!(p.namespace(), Namespace::Knowledge);
        assert_eq!(p.segments(), &["docs", "api"]);
        assert_eq!(p.name(), Some("api"));
    }

    #[test]
    fn test_parse_pathway_without_protocol() {
        let p = Pathway::parse("knowledge/docs/api").unwrap();
        assert_eq!(p.namespace(), Namespace::Knowledge);
        assert_eq!(p.segments(), &["docs", "api"]);
    }

    #[test]
    fn test_parse_pathway_root() {
        let p = Pathway::parse("a3s://knowledge").unwrap();
        assert_eq!(p.namespace(), Namespace::Knowledge);
        assert!(p.segments().is_empty());
        assert!(p.is_root());
    }

    #[test]
    fn test_parse_invalid_pathway() {
        assert!(Pathway::parse("").is_err());
        assert!(Pathway::parse("a3s://").is_err());
        assert!(Pathway::parse("a3s://invalid_namespace").is_err());
    }

    #[test]
    fn test_pathway_parent() {
        let p = Pathway::parse("a3s://knowledge/docs/api").unwrap();
        let parent = p.parent().unwrap();
        assert_eq!(parent.segments(), &["docs"]);

        let root = Pathway::parse("a3s://knowledge").unwrap();
        assert!(root.parent().is_none());
    }

    #[test]
    fn test_pathway_join() {
        let p = Pathway::parse("a3s://knowledge/docs").unwrap();
        let child = p.join("api");
        assert_eq!(child.segments(), &["docs", "api"]);
    }

    #[test]
    fn test_pathway_display() {
        let p = Pathway::parse("a3s://memory/user/prefs").unwrap();
        assert_eq!(p.to_string(), "a3s://memory/user/prefs");
    }

    #[test]
    fn test_pathway_is_prefix_of() {
        let parent = Pathway::parse("a3s://knowledge/docs").unwrap();
        let child = Pathway::parse("a3s://knowledge/docs/api").unwrap();
        let other = Pathway::parse("a3s://memory/user").unwrap();

        assert!(parent.is_prefix_of(&child));
        assert!(!child.is_prefix_of(&parent));
        assert!(!parent.is_prefix_of(&other));
    }

    #[test]
    fn test_pathway_depth() {
        let root = Pathway::parse("a3s://knowledge").unwrap();
        let level1 = Pathway::parse("a3s://knowledge/docs").unwrap();
        let level2 = Pathway::parse("a3s://knowledge/docs/api").unwrap();

        assert_eq!(root.depth(), 0);
        assert_eq!(level1.depth(), 1);
        assert_eq!(level2.depth(), 2);
    }

    #[test]
    fn test_pathway_constructors() {
        let k = Pathway::knowledge("docs/api").unwrap();
        assert_eq!(k.namespace(), Namespace::Knowledge);
        assert_eq!(k.segments(), &["docs", "api"]);

        let m = Pathway::memory("user/prefs").unwrap();
        assert_eq!(m.namespace(), Namespace::Memory);

        let c = Pathway::capability("tools/search").unwrap();
        assert_eq!(c.namespace(), Namespace::Capability);
    }

    #[test]
    fn test_pathway_root_constructor() {
        let root = Pathway::root(Namespace::Knowledge);
        assert_eq!(root.namespace(), Namespace::Knowledge);
        assert!(root.is_root());
        assert_eq!(root.depth(), 0);
    }
}

