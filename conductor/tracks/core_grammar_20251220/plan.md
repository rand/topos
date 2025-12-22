# Plan: Core Grammar and Validation

## Phase 1: Grammar Definition
- [x] Task: Define basic Topos grammar in `tree-sitter-topos/grammar.js` c6ff3b8
- [x] Task: Implement corpus tests for top-level sections a1c3e82
- [ ] Task: Implement grammar for Requirements and Concepts
- [ ] Task: Implement grammar for Tasks and Typed Holes
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Grammar Definition' (Protocol in workflow.md)

## Phase 2: Parser and Analysis Integration
- [ ] Task: Generate parser and verify linkage in `topos-analysis`
- [ ] Task: Implement AST traversal logic to extract diagnostics
- [ ] Task: Implement basic traceability check (Task -> Requirement)
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Parser and Analysis Integration' (Protocol in workflow.md)

## Phase 3: CLI Command
- [ ] Task: Implement `topos check` command in `topos-cli`
- [ ] Task: Add integration tests for `topos check` with valid/invalid files
- [ ] Task: Conductor - User Manual Verification 'Phase 3: CLI Command' (Protocol in workflow.md)
