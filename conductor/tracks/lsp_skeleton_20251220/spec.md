# Spec: LSP Skeleton and Workspace Solidification

## Overview
This track focuses on initializing the Rust workspace defined in the `Cargo.toml` and setting up a minimal, functional Language Server Protocol (LSP) server using `tower-lsp-server`.

## Requirements
- Initialize all crates defined in `Cargo.toml` members.
- Implement a basic LSP server that can start and respond to `initialize` and `initialized` requests.
- Integrate with `tree-sitter-topos` for basic syntax validation.
- Provide a CLI entry point to start the LSP server.

## Success Criteria
- `cargo build` passes for the entire workspace.
- `topos lsp` starts a server that can communicate over stdio.
- Basic "hover" or "diagnostics" placeholder is visible in an LSP client (e.g., VS Code).
