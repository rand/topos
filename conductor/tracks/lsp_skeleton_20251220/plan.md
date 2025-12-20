# Plan: LSP Skeleton and Workspace Solidification

## Phase 1: Workspace Initialization [checkpoint: 94fd5b4]
- [x] Task: Create directory structure for all workspace crates (`crates/topos-*`) 9c8ec6d
- [x] Task: Initialize `Cargo.toml` for each crate with basic dependencies 9067056
- [x] Task: Conductor - User Manual Verification 'Phase 1: Workspace Initialization' (Protocol in workflow.md) 94fd5b4

## Phase 2: LSP Server Skeleton [checkpoint: cc8041f]
- [x] Task: Implement `ToposServer` struct in `topos-lsp` using `tower-lsp-server` fa4ffa0
- [x] Task: Write unit tests for LSP server initialization 1334256
- [x] Task: Implement stdio transport in `topos-cli` to launch the LSP server c3339d3
- [x] Task: Conductor - User Manual Verification 'Phase 2: LSP Server Skeleton' (Protocol in workflow.md) cc8041f

## Phase 3: Basic Syntax Integration
- [x] Task: Integrate `tree-sitter-topos` into `topos-analysis` 674b6da
- [x] Task: Implement a basic "check" function that uses tree-sitter to find syntax errors b811e67
- [x] Task: Connect the check function to LSP diagnostics 0e6ecc9
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Basic Syntax Integration' (Protocol in workflow.md)
