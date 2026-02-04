use a3s_context::{A3SClient, Config};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "a3s-ctx")]
#[command(about = "A3S Context - Autonomous Agent Adaptive Storage", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest content into A3S
    Ingest {
        /// Source path (file or directory)
        source: String,

        /// Target pathway
        #[arg(short, long)]
        target: String,
    },

    /// Query the context store
    Query {
        /// Query text
        query: String,

        /// Result limit
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// List nodes at a pathway
    List {
        /// Pathway to list
        pathway: String,
    },

    /// Read a node's content
    Read {
        /// Pathway to read
        pathway: String,

        /// Show only brief summary
        #[arg(short, long)]
        brief: bool,

        /// Show only summary
        #[arg(short, long)]
        summary: bool,
    },

    /// Remove a node
    Remove {
        /// Pathway to remove
        pathway: String,

        /// Remove recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Show storage statistics
    Stats,

    /// Initialize storage
    Init,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(cli.log_level)
        .init();

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        Config::from_file(&config_path)?
    } else {
        Config::from_env()
    };

    // Create client
    let client = A3SClient::new(config).await?;

    match cli.command {
        Commands::Ingest { source, target } => {
            println!("Ingesting {} into {}...", source, target);
            let result = client.ingest(&source, &target).await?;
            println!(
                "✓ Created: {}, Updated: {}, Errors: {}",
                result.nodes_created,
                result.nodes_updated,
                result.errors.len()
            );
            if !result.errors.is_empty() {
                println!("\nErrors:");
                for err in result.errors {
                    println!("  - {}", err);
                }
            }
        }

        Commands::Query { query, limit } => {
            println!("Searching for: {}", query);
            let result = client
                .query_with_options(
                    &query,
                    a3s_context::QueryOptions {
                        limit: Some(limit),
                        ..Default::default()
                    },
                )
                .await?;

            println!(
                "\nFound {} results (searched {} nodes in {}ms):\n",
                result.matches.len(),
                result.total_searched,
                result.search_time_ms
            );

            for (i, m) in result.matches.iter().enumerate() {
                println!("{}. {} (score: {:.3})", i + 1, m.pathway, m.score);
                println!("   {}", m.brief);
                println!();
            }
        }

        Commands::List { pathway } => {
            let nodes = client.list(&pathway).await?;
            println!("Nodes at {}:\n", pathway);
            for node in nodes {
                let kind_str = format!("{:?}", node.kind);
                println!(
                    "  {} {} ({})",
                    if node.is_directory { "📁" } else { "📄" },
                    node.pathway.name().unwrap_or(""),
                    kind_str
                );
            }
        }

        Commands::Read {
            pathway,
            brief,
            summary,
        } => {
            if brief {
                let content = client.brief(&pathway).await?;
                println!("{}", content);
            } else if summary {
                let content = client.summary(&pathway).await?;
                println!("{}", content);
            } else {
                let node = client.read(&pathway).await?;
                println!("{}", node.content);
            }
        }

        Commands::Remove { pathway, recursive } => {
            client.remove(&pathway, recursive).await?;
            println!("✓ Removed {}", pathway);
        }

        Commands::Stats => {
            let stats = client.stats().await?;
            println!("Storage Statistics:");
            println!("  Total nodes: {}", stats.total_nodes);
            println!("  Total directories: {}", stats.total_directories);
            println!("  Total size: {} bytes", stats.total_size_bytes);
        }

        Commands::Init => {
            println!("✓ Storage initialized");
        }
    }

    client.shutdown().await?;

    Ok(())
}
