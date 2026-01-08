# Spec: Core Grammar and Validation

## Overview
This track focuses on transforming the placeholder `tree-sitter-topos` into a functional parser for the Topos language and implementing the `topos check` command to provide user feedback.

## Requirements
- **Grammar Implementation:** Define `grammar.js` for Topos including `spec`, `Principles`, `Requirements`, `Concepts`, `Tasks`, `Typed Holes`, and foreign code blocks.
- **Parser Generation:** Ensure `tree-sitter generate` and `cargo build` work correctly.
- **Validation Logic:** Implement `topos-analysis` logic to traverse the Tree-sitter AST and find `ERROR` or `MISSING` nodes.
- **Traceability Check:** Add basic verification that `TASK` blocks reference valid `REQ` IDs.
- **CLI Implementation:** Implement `topos check <file>` in `topos-cli`.

## Success Criteria
- `cargo run -- check examples/valid.tps` exits with 0.
- `cargo run -- check examples/invalid.tps` exits with non-zero and prints diagnostics.
- Tree-sitter corpus tests pass.
