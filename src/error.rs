//! Error types for A3S Context

use thiserror::Error;

pub type Result<T> = std::result::Result<T, A3SError>;

#[derive(Error, Debug)]
pub enum A3SError {
    #[error("Invalid pathway: {0}")]
    InvalidPathway(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Directory not empty: {0}")]
    DirectoryNotEmpty(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Digest generation error: {0}")]
    DigestGeneration(String),

    #[error("Ingest error: {0}")]
    Ingest(String),

    #[error("Retrieval error: {0}")]
    Retrieval(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Not initialized")]
    NotInitialized,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<String> for A3SError {
    fn from(s: String) -> Self {
        A3SError::Internal(s)
    }
}

impl From<&str> for A3SError {
    fn from(s: &str) -> Self {
        A3SError::Internal(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = A3SError::InvalidPathway("test".to_string());
        assert_eq!(err.to_string(), "Invalid pathway: test");

        let err = A3SError::NodeNotFound("a3s://knowledge/test".to_string());
        assert_eq!(err.to_string(), "Node not found: a3s://knowledge/test");

        let err = A3SError::NotInitialized;
        assert_eq!(err.to_string(), "Not initialized");
    }

    #[test]
    fn test_error_from_string() {
        let err: A3SError = "test error".into();
        assert!(matches!(err, A3SError::Internal(_)));
        assert_eq!(err.to_string(), "Internal error: test error");
    }

    #[test]
    fn test_error_from_owned_string() {
        let err: A3SError = String::from("owned error").into();
        assert!(matches!(err, A3SError::Internal(_)));
        assert_eq!(err.to_string(), "Internal error: owned error");
    }

    #[test]
    fn test_error_variants() {
        // Test all error variants can be created
        let _ = A3SError::InvalidPathway("test".to_string());
        let _ = A3SError::NodeNotFound("test".to_string());
        let _ = A3SError::DirectoryNotEmpty("test".to_string());
        let _ = A3SError::AlreadyExists("test".to_string());
        let _ = A3SError::Storage("test".to_string());
        let _ = A3SError::Embedding("test".to_string());
        let _ = A3SError::DigestGeneration("test".to_string());
        let _ = A3SError::Ingest("test".to_string());
        let _ = A3SError::Retrieval("test".to_string());
        let _ = A3SError::Session("test".to_string());
        let _ = A3SError::Config("test".to_string());
        let _ = A3SError::NotInitialized;
        let _ = A3SError::Internal("test".to_string());
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: A3SError = io_err.into();
        assert!(matches!(err, A3SError::Io(_)));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(A3SError::Internal("test".to_string()))
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }
}
