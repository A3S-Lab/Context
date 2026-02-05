//! Integration tests for A3S Context

use a3s_context::config::RerankConfig;
use a3s_context::rerank::{MockReranker, RerankDocument, Reranker};
use a3s_context::{A3SClient, Config, Namespace, Pathway};

fn create_test_config() -> Config {
    let mut config = Config::default();
    // Use mock embedder for testing (no API key required)
    config.embedding.provider = "mock".to_string();
    config.llm.auto_digest = false; // Disable LLM digest generation in tests
    config
}

fn create_test_config_with_rerank() -> Config {
    let mut config = create_test_config();
    config.retrieval.rerank = true;
    config.retrieval.rerank_config = RerankConfig {
        provider: "mock".to_string(),
        api_base: None,
        api_key: None,
        model: None,
        top_n: Some(5),
    };
    config
}

#[tokio::test]
async fn test_client_initialization() {
    let config = create_test_config();
    let result = A3SClient::new(config).await;
    assert!(
        result.is_ok(),
        "Client initialization failed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_pathway_operations() {
    // Test pathway parsing
    let p = Pathway::parse("a3s://knowledge/docs/api").unwrap();
    assert_eq!(p.namespace(), Namespace::Knowledge);
    assert_eq!(p.segments(), &["docs", "api"]);

    // Test pathway parent
    let parent = p.parent().unwrap();
    assert_eq!(parent.segments(), &["docs"]);

    // Test pathway join
    let child = parent.join("test");
    assert_eq!(child.segments(), &["docs", "test"]);
}

#[tokio::test]
async fn test_namespace_operations() {
    assert_eq!(Namespace::Knowledge.as_str(), "knowledge");
    assert_eq!(Namespace::from_str("knowledge"), Some(Namespace::Knowledge));
    assert_eq!(Namespace::from_str("invalid"), None);
}

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.log_level, "info");
    assert!(config.llm.auto_digest);
    assert_eq!(config.retrieval.default_limit, 10);
}

#[test]
fn test_config_from_env() {
    std::env::set_var("A3S_LOG_LEVEL", "debug");
    let config = Config::from_env();
    assert_eq!(config.log_level, "debug");
    std::env::remove_var("A3S_LOG_LEVEL");
}

#[tokio::test]
async fn test_client_with_mock_embedder() {
    let config = create_test_config();
    let client = A3SClient::new(config).await.unwrap();

    // Test that we can get stats
    let stats = client.stats().await.unwrap();
    assert_eq!(stats.total_nodes, 0);
}

#[tokio::test]
async fn test_client_with_rerank_enabled() {
    let config = create_test_config_with_rerank();
    let result = A3SClient::new(config).await;
    assert!(
        result.is_ok(),
        "Client with rerank should initialize: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_mock_reranker_integration() {
    let reranker = MockReranker::new();
    let documents = vec![
        RerankDocument {
            id: "doc1".to_string(),
            text: "The capital of France is Paris.".to_string(),
        },
        RerankDocument {
            id: "doc2".to_string(),
            text: "Python is a programming language.".to_string(),
        },
        RerankDocument {
            id: "doc3".to_string(),
            text: "Paris is known for the Eiffel Tower.".to_string(),
        },
    ];

    let results = reranker
        .rerank("What is the capital of France?", documents, 2)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    // Results should be sorted by score
    assert!(results[0].score >= results[1].score);
}

#[test]
fn test_rerank_config_default() {
    let config = RerankConfig::default();
    assert_eq!(config.provider, "mock");
    assert!(config.api_base.is_none());
    assert!(config.api_key.is_none());
    assert!(config.model.is_none());
    assert!(config.top_n.is_none());
}

#[test]
fn test_rerank_config_custom() {
    let config = RerankConfig {
        provider: "cohere".to_string(),
        api_base: Some("https://custom.api".to_string()),
        api_key: Some("test-key".to_string()),
        model: Some("rerank-english-v3.0".to_string()),
        top_n: Some(10),
    };

    assert_eq!(config.provider, "cohere");
    assert_eq!(config.api_base, Some("https://custom.api".to_string()));
    assert_eq!(config.api_key, Some("test-key".to_string()));
    assert_eq!(config.model, Some("rerank-english-v3.0".to_string()));
    assert_eq!(config.top_n, Some(10));
}
