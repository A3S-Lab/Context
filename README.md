# A3S Context

<p align="center">
  <strong>Hierarchical Context Management for AI Agents</strong>
</p>

<p align="center">
  <em>Utility layer — semantic search, multi-level digests, and flexible storage backends</em>
</p>

<p align="center">
  <a href="#key-features">Features</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#development">Development</a>
</p>

---

## Overview

**A3S Context** provides hierarchical context management with automatic summarization, semantic search, and intelligent retrieval. Designed for AI agents that need to efficiently manage large amounts of contextual information.

## Key Features

- **Hierarchical Organization**: URI-like pathways (`a3s://namespace/path/to/node`) for intuitive context organization
- **Multi-Level Digests**: Automatic generation of brief/summary/full content levels for efficient retrieval
- **Semantic Search**: Vector-based similarity search with hierarchical exploration
- **Flexible Storage**: Local file-based or in-memory storage backends
- **Namespace Isolation**: Separate namespaces for knowledge, memory, capabilities, and sessions
- **Async-First**: Built on Tokio for high-performance concurrent operations

## Architecture

### Core Concepts

1. **Pathway**: URI-like addressing scheme
   - Format: `a3s://namespace/path/to/node`
   - Examples: `a3s://knowledge/docs/api`, `a3s://memory/user/preferences`

2. **Namespaces**: Logical grouping of context
   - `knowledge`: Documents, code, and other knowledge base content
   - `memory`: User and agent memories
   - `capability`: Agent capabilities and tools
   - `session`: Active conversation sessions

3. **Digest Levels**: Multi-level summarization
   - **Brief** (~50 tokens): Quick relevance check
   - **Summary** (~500 tokens): Planning and understanding
   - **Full**: Complete original content

4. **Node Types**: Different kinds of content
   - Document, Code, Markdown, Memory, Capability, Message, Data

## Installation

```bash
cargo install a3s_context
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
a3s_context = "0.1"
```

## Quick Start

### As a Library

```rust
use a3s_context::{A3SClient, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client with default config
    let client = A3SClient::new(Config::default()).await?;

    // Ingest documents
    client.ingest("./docs", "a3s://knowledge/docs").await?;

    // Query
    let results = client.query("How does authentication work?").await?;

    for result in results.matches {
        println!("{}: {}", result.pathway, result.brief);
    }

    Ok(())
}
```

### As a CLI Tool

```bash
# Initialize storage
a3s-ctx init

# Ingest content
a3s-ctx ingest ./docs --target a3s://knowledge/docs

# Query
a3s-ctx query "How does authentication work?" --limit 5

# List nodes
a3s-ctx list a3s://knowledge/docs

# Read content
a3s-ctx read a3s://knowledge/docs/api.md --brief

# Show statistics
a3s-ctx stats
```

## Configuration

### Configuration File

Create `a3s.yaml`:

```yaml
storage:
  backend: local
  path: ./a3s_data
  vector_index:
    index_type: hnsw
    hnsw_m: 16
    hnsw_ef_construction: 200

embedding:
  provider: openai
  model: text-embedding-3-small
  dimension: 1536
  batch_size: 32

llm:
  provider: openai
  model: gpt-4
  auto_digest: true

retrieval:
  default_limit: 10
  score_threshold: 0.5
  hierarchical: true
  max_depth: 3
  rerank: true                    # Enable reranking
  rerank_config:
    provider: cohere              # cohere, jina, openai, mock
    model: rerank-english-v3.0    # Model name (optional)
    top_n: 10                     # Top N results after reranking

ingest:
  max_file_size: 10485760  # 10MB
  chunk_size: 1000
  chunk_overlap: 200
  ignore_patterns:
    - .git
    - node_modules
    - target

log_level: info
```

### Environment Variables

```bash
# Storage
export A3S_STORAGE_PATH=./a3s_data

# Embedding
export A3S_EMBEDDING_API_BASE=https://api.openai.com/v1
export A3S_EMBEDDING_API_KEY=your-api-key
export A3S_EMBEDDING_MODEL=text-embedding-3-small

# LLM for digest generation
export A3S_LLM_API_BASE=https://api.openai.com/v1
export A3S_LLM_API_KEY=your-api-key
export A3S_LLM_MODEL=gpt-4

# Reranking
export A3S_RERANK_PROVIDER=cohere    # cohere, jina, openai, mock
export A3S_RERANK_API_BASE=https://api.cohere.ai/v1
export A3S_RERANK_API_KEY=your-api-key
export A3S_RERANK_MODEL=rerank-english-v3.0
export A3S_RERANK_TOP_N=10

# Provider-specific API keys (fallback)
export COHERE_API_KEY=your-cohere-key
export JINA_API_KEY=your-jina-key
export OPENAI_API_KEY=your-openai-key

# Logging
export A3S_LOG_LEVEL=info
```

## API Reference

### Client Operations

```rust
// Ingest content
let result = client.ingest("./docs", "a3s://knowledge/docs").await?;

// Query with options
let results = client.query_with_options(
    "search query",
    QueryOptions {
        namespace: Some(Namespace::Knowledge),
        limit: Some(10),
        threshold: Some(0.7),
        include_content: false,
        pathway_filter: Some("docs/*"),
    }
).await?;

// List nodes
let nodes = client.list("a3s://knowledge/docs").await?;

// Read content
let node = client.read("a3s://knowledge/docs/api.md").await?;
let brief = client.brief("a3s://knowledge/docs/api.md").await?;
let summary = client.summary("a3s://knowledge/docs/api.md").await?;

// Remove
client.remove("a3s://knowledge/docs/old", true).await?;

// Session management
let session = client.session(None).await?;
session.add_message(MessageRole::User, "Hello".to_string());
session.commit().await?;

// Statistics
let stats = client.stats().await?;
```

## Project Structure

```
a3s-context/
├── src/
│   ├── lib.rs              # Main library interface
│   ├── main.rs             # CLI binary
│   ├── core.rs             # Core data structures
│   ├── pathway.rs          # Pathway addressing
│   ├── digest.rs           # Multi-level digest generation
│   ├── error.rs            # Error types
│   ├── config.rs           # Configuration
│   ├── embedding.rs        # Embedding models
│   ├── ingest.rs           # Content ingestion
│   ├── retrieval.rs        # Hierarchical retrieval
│   ├── session.rs          # Session management
│   ├── rerank/             # Reranking module
│   │   ├── mod.rs          # Reranker trait and factory
│   │   ├── mock.rs         # Mock reranker for testing
│   │   ├── cohere.rs       # Cohere Rerank API
│   │   ├── jina.rs         # Jina Reranker API
│   │   └── openai.rs       # OpenAI pointwise reranking
│   └── storage/
│       ├── mod.rs          # Storage abstraction
│       ├── local.rs        # Local file storage
│       ├── memory.rs       # In-memory storage
│       └── vector_index.rs # Vector index
├── examples/               # Usage examples
├── tests/                  # Integration tests
├── benches/                # Benchmarks
├── Cargo.toml
└── README.md
```

## Performance

- **Async I/O**: Non-blocking operations for high concurrency
- **Efficient Indexing**: HNSW-based vector index for fast similarity search
- **Caching**: In-memory caching of frequently accessed nodes
- **Batch Operations**: Batch embedding and storage operations

## Development

### Dependencies

| Dependency | Install | Purpose |
|------------|---------|---------|
| `cargo-llvm-cov` | `cargo install cargo-llvm-cov` | Code coverage (optional) |
| `lcov` | `brew install lcov` / `apt install lcov` | Coverage report formatting (optional) |
| `cargo-watch` | `cargo install cargo-watch` | File watching (optional) |

### Build Commands

```bash
# Build
just build                   # Release build
just build-debug             # Debug build

# Test (with colored progress display)
just test                    # All tests with pretty output
just test-raw                # Raw cargo output
just test-v                  # Verbose output (--nocapture)
just test-one TEST           # Run specific test

# Test subsets
just test-pathway            # Pathway module tests
just test-storage            # Storage module tests
just test-retrieval          # Retrieval module tests
just test-session            # Session module tests
just test-config             # Config module tests
just test-integration        # Integration tests

# Coverage (requires cargo-llvm-cov + lcov)
just test-cov                # Pretty coverage with progress
just cov                     # Terminal coverage report
just cov-html                # HTML report (opens in browser)
just cov-table               # File-by-file table
just cov-ci                  # Generate lcov.info for CI
just cov-module pathway      # Coverage for specific module
just cov-clean               # Clean coverage data

# Format & Lint
just fmt                     # Format code
just fmt-check               # Check formatting
just lint                    # Clippy lint
just ci                      # Full CI checks (fmt + lint + test)

# CLI
just run <args>              # Run the CLI tool

# Utilities
just check                   # Fast compile check
just bench                   # Run benchmarks
just watch                   # Watch and run tests
just doc                     # Generate and open docs
just stats                   # Show project statistics
just clean                   # Clean build artifacts
just update                  # Update dependencies
just install                 # Install the binary
```

## A3S Ecosystem

A3S Context is a **utility component** of the A3S ecosystem — a standalone hierarchical context system that can be used by any application needing organized memory/knowledge retrieval.

```
┌──────────────────────────────────────────────────────────┐
│                    A3S Ecosystem                         │
│                                                          │
│  Infrastructure:  a3s-box     (MicroVM sandbox runtime)  │
│                      │                                   │
│  Application:     a3s-code    (AI coding agent)          │
│                    /   \                                 │
│  Utilities:   a3s-lane  a3s-context                     │
│               (queue)        ▲                           │
│                              │                           │
│                        You are here                      │
└──────────────────────────────────────────────────────────┘
```

| Project | Package | Relationship |
|---------|---------|--------------|
| **box** | `a3s-box-*` | Defines `ContextProvider` trait that `context` can implement |
| **code** | `a3s-code` | Uses `context` for memory storage and semantic retrieval |
| **lane** | `a3s-lane` | Independent utility (no direct relationship) |

**Standalone Usage**: `a3s-context` works independently for any hierarchical data organization:
- Documentation search systems
- RAG (Retrieval-Augmented Generation) backends
- Knowledge base management
- Any system needing multi-level summarization with semantic search

## Roadmap

### Phase 1: Core ✅

- [x] Hierarchical node organization with pathways
- [x] Multi-level digest system (Brief/Summary/Full)
- [x] Vector-based semantic search
- [x] Local and in-memory storage backends
- [x] Namespace isolation (knowledge, memory, capability, session)
- [x] Session management
- [x] Configuration system (YAML/TOML/JSON)
- [x] CLI tool

### Phase 2: Search Enhancement 🚧

- [x] Reranking support (Cohere, Jina, OpenAI, Mock providers)
- [ ] Intent analyzer (multi-condition query decomposition)
- [ ] Score propagation (directory score = weighted child scores)
- [ ] Convergence detection (stop after N rounds with unchanged topk)
- [ ] Sparse + dense hybrid search (BM25 + vector)
- [ ] Query expansion and reformulation
- [ ] Glob/Find API (`glob(pattern, uri)`, `find(query, target_uri)`)
- [ ] Volcengine Rerank provider (doubao-seed-rerank)

### Phase 3: Ecosystem Integration 📋

- [ ] Remote storage backend (gRPC-based)
- [ ] Implement `a3s-box-core::ContextProvider` trait
- [ ] Native integration with `a3s-code` for agent memory
- [ ] Python bindings (PyO3)
- [ ] Web UI for visualization
- [ ] REST API server mode
- [ ] Retrieval trajectory visualization

### Phase 4: Advanced Features 📋

- [ ] VLM support (Vision Language Model for image understanding)
- [ ] Automatic memory extraction (session -> long-term memory)
- [ ] Distributed deployment support
- [ ] Advanced memory management (decay, consolidation)
- [ ] Plugin system for custom parsers
- [ ] Multi-tenant support

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see LICENSE file for details
