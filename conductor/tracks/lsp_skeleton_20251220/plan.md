# Plan: LSP Skeleton and Workspace Solidification

## Phase 1: Workspace Initialization
- [x] Task: Create directory structure for all workspace crates (`crates/topos-*`) 9c8ec6d
- [x] Task: Initialize `Cargo.toml` for each crate with basic dependencies 9067056
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Workspace Initialization' (Protocol in workflow.md)
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Workspace Initialization' (Protocol in workflow.md)

## Phase 2: LSP Server Skeleton
- [ ] Task: Implement `ToposServer` struct in `topos-lsp` using `tower-lsp-server`
- [ ] Task: Write unit tests for LSP server initialization
- [ ] Task: Implement stdio transport in `topos-cli` to launch the LSP server
- [ ] Task: Conductor - User Manual Verification 'Phase 2: LSP Server Skeleton' (Protocol in workflow.md)

## Phase 3: Basic Syntax Integration
- [ ] Task: Integrate `tree-sitter-topos` into `topos-analysis`
- [ ] Task: Implement a basic "check" function that uses tree-sitter to find syntax errors
- [ ] Task: Connect the check function to LSP diagnostics
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Basic Syntax Integration' (Protocol in workflow.md)
