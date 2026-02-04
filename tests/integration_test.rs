//! Integration tests for A3S Context

use a3s_context::{A3SClient, Config, Namespace, Pathway};

fn create_test_config() -> Config {
    let mut config = Config::default();
    // Use mock embedder for testing (no API key required)
    config.embedding.provider = "mock".to_string();
    config.llm.auto_digest = false; // Disable LLM digest generation in tests
    config
}

#[tokio::test]
async fn test_client_initialization() {
    let config = create_test_config();
    let result = A3SClient::new(config).await;
    assert!(result.is_ok(), "Client initialization failed: {:?}", result.err());
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
