//! Session management for conversation tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::config::Config;
use crate::embedding::Embedder;
use crate::error::Result;
use crate::pathway::Pathway;
use crate::storage::StorageBackend;

/// A conversation session
#[derive(Clone)]
pub struct Session {
    id: String,
    #[allow(dead_code)]
    user: String,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
    messages: Vec<Message>,
    #[allow(dead_code)]
    storage: Arc<dyn StorageBackend>,
    #[allow(dead_code)]
    embedder: Arc<dyn Embedder>,
    #[allow(dead_code)]
    config: Config,
}

impl Session {
    pub async fn new(
        id: Option<&str>,
        storage: Arc<dyn StorageBackend>,
        embedder: Arc<dyn Embedder>,
        config: &Config,
    ) -> Result<Self> {
        let id = id
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        Ok(Self {
            id,
            user: "default".to_string(),
            created_at: Utc::now(),
            messages: Vec::new(),
            storage,
            embedder,
            config: config.clone(),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) {
        self.messages.push(Message {
            role,
            content,
            timestamp: Utc::now(),
            contexts_used: Vec::new(),
        });
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub async fn commit(&mut self) -> Result<()> {
        // Save session to storage
        let _pathway = Pathway::parse(&format!("a3s://session/{}", self.id))?;

        // TODO: Implement session persistence

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub contexts_used: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, VectorIndexConfig};
    use crate::embedding::MockEmbedder;
    use crate::storage::MemoryStorage;

    fn create_test_embedder() -> Arc<dyn Embedder> {
        // Use mock embedder for testing (no API key required)
        Arc::new(MockEmbedder::new(128))
    }

    fn create_test_storage() -> Arc<dyn StorageBackend> {
        let config = VectorIndexConfig {
            index_type: "hnsw".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
        };
        Arc::new(MemoryStorage::new(&config))
    }

    #[tokio::test]
    async fn test_session_new_with_id() {
        let storage = create_test_storage();
        let embedder = create_test_embedder();
        let config = Config::default();

        let session = Session::new(Some("test-session-id"), storage, embedder, &config)
            .await
            .unwrap();

        assert_eq!(session.id(), "test-session-id");
        assert_eq!(session.messages().len(), 0);
    }

    #[tokio::test]
    async fn test_session_new_without_id() {
        let storage = create_test_storage();
        let embedder = create_test_embedder();
        let config = Config::default();

        let session = Session::new(None, storage, embedder, &config)
            .await
            .unwrap();

        assert!(!session.id().is_empty());
        assert_eq!(session.messages().len(), 0);
    }

    #[tokio::test]
    async fn test_session_add_message() {
        let storage = create_test_storage();
        let embedder = create_test_embedder();
        let config = Config::default();

        let mut session = Session::new(None, storage, embedder, &config)
            .await
            .unwrap();

        session.add_message(MessageRole::User, "Hello".to_string());
        session.add_message(MessageRole::Assistant, "Hi there!".to_string());

        assert_eq!(session.messages().len(), 2);
        assert_eq!(session.messages()[0].role, MessageRole::User);
        assert_eq!(session.messages()[0].content, "Hello");
        assert_eq!(session.messages()[1].role, MessageRole::Assistant);
        assert_eq!(session.messages()[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_session_commit() {
        let storage = create_test_storage();
        let embedder = create_test_embedder();
        let config = Config::default();

        let mut session = Session::new(None, storage, embedder, &config)
            .await
            .unwrap();

        session.add_message(MessageRole::User, "Test message".to_string());

        // Commit should not fail
        let result = session.commit().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_role_serialization() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let role = MessageRole::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");

        let role = MessageRole::System;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"system\"");
    }

    #[test]
    fn test_message_role_deserialization() {
        let role: MessageRole = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(role, MessageRole::User);

        let role: MessageRole = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, MessageRole::Assistant);

        let role: MessageRole = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(role, MessageRole::System);
    }

    #[test]
    fn test_message_serialization() {
        let message = Message {
            role: MessageRole::User,
            content: "Test content".to_string(),
            timestamp: Utc::now(),
            contexts_used: vec!["a3s://knowledge/test".to_string()],
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Test content\""));
    }
}
