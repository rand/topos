# Technology Stack

## Core Language & Runtime
- **Language:** Rust (Edition 2024, v1.93+)
- **Async Runtime:** `tokio` (v1.40+) - The industry standard for async Rust.
- **Wasm Support:** `wasm-bindgen` - For running the compiler and LSP in browser environments (VS Code Web).

## Compiler & Analysis
- **Incremental Computation:** `salsa` (v0.18) - Provides the query-based architecture for instant re-compilation.
- **Parsing:** `tree-sitter` (v0.25) - Delivers sub-millisecond parsing with robust error recovery.
- **Reflection System:** `facet.rs` (v0.28) - The core engine for semantic reflection, diffing, and serialization.
- **Vector Search:** `lance` - Enables high-performance semantic search for the Context Compiler.

## Protocols
- **Language Server Protocol (LSP):** `tower-lsp-server` (v0.1) - Modern, async implementation of the LSP specification.
- **Model Context Protocol (MCP):** `rmcp` (v0.8) - Official Rust SDK for integrating with LLMs.

## Command Line & Interface
- **CLI Argument Parsing:** `clap` (v4.5) - Standard, derive-based argument parsing for the `topos` binary.
- **Terminal UI (TUI):** `ratatui` - Powered rich, interactive terminal interfaces (e.g., for live drift monitoring).
- **Output Styling:** `colored` / `proctitle` - For clean, readable terminal output.

## Quality & Observability
- **Testing:** `insta` (Snapshot testing) & `proptest` (Property-based testing).
- **Observability:** `tracing`, `tracing-opentelemetry` & `opentelemetry-otlp` - For deep introspection of compiler performance and query graphs.
