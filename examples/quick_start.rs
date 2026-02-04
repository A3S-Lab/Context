use a3s_context::{A3SClient, Config, QueryOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== A3S Context Quick Start Example ===\n");

    // Create client with default configuration
    let config = Config::default();
    let client = A3SClient::new(config).await?;

    println!("✓ Client initialized\n");

    // Example 1: Ingest some sample content
    println!("1. Ingesting sample content...");

    // Create a temporary directory with sample files
    let temp_dir = tempfile::tempdir()?;
    let sample_path = temp_dir.path();

    std::fs::write(
        sample_path.join("readme.md"),
        "# Sample Project\n\nThis is a sample project for testing A3S Context.",
    )?;

    std::fs::write(
        sample_path.join("api.md"),
        "# API Documentation\n\n## Authentication\n\nUse API keys for authentication.",
    )?;

    std::fs::write(
        sample_path.join("code.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}",
    )?;

    let result = client
        .ingest(
            sample_path.to_str().unwrap(),
            "a3s://knowledge/sample_project",
        )
        .await?;

    println!(
        "   ✓ Created {} nodes, updated {} nodes\n",
        result.nodes_created, result.nodes_updated
    );

    // Example 2: List ingested content
    println!("2. Listing ingested content...");
    let nodes = client.list("a3s://knowledge/sample_project").await?;

    for node in &nodes {
        println!(
            "   {} {} ({:?})",
            if node.is_directory { "📁" } else { "📄" },
            node.pathway.name().unwrap_or(""),
            node.kind
        );
    }
    println!();

    // Example 3: Query the content
    println!("3. Querying: 'authentication'...");
    let query_result = client
        .query_with_options(
            "authentication",
            QueryOptions {
                limit: Some(3),
                ..Default::default()
            },
        )
        .await?;

    println!(
        "   Found {} results in {}ms:\n",
        query_result.matches.len(),
        query_result.search_time_ms
    );

    for (i, m) in query_result.matches.iter().enumerate() {
        println!("   {}. {} (score: {:.3})", i + 1, m.pathway, m.score);
        println!("      {}\n", m.brief);
    }

    // Example 4: Read specific content
    if let Some(first_match) = query_result.matches.first() {
        println!("4. Reading brief summary of top result...");
        let brief = client.brief(&first_match.pathway.to_string()).await?;
        println!("   {}\n", brief);
    }

    // Example 5: Get statistics
    println!("5. Storage statistics:");
    let stats = client.stats().await?;
    println!("   Total nodes: {}", stats.total_nodes);
    println!("   Total size: {} bytes\n", stats.total_size_bytes);

    // Cleanup
    client.shutdown().await?;
    println!("✓ Example completed successfully!");

    Ok(())
}
