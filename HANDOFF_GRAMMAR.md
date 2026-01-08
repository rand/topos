# Handover Document: Core Grammar and Validation Track

## Context
This document describes the state of the "Core Grammar and Validation" track for the Topos project. The goal is to implement a Tree-sitter grammar for the Topos language and a CLI command `topos check` to validate files.

## Current Status
- **Phase 1 (Grammar Definition):**
  - Implemented `grammar.js` with rules for `spec`, `import`, `requirement`, `concept`, `task`, `behavior`, `invariant`, `aesthetic`, `hole`.
  - Implemented `scanner.c` to handle Python-style indentation (`INDENT`, `DEDENT`), line-based parsing (`NEWLINE`), and context-sensitive `PROSE` tokens (to distinguish keywords from free text).
  - Corpus tests created in `tree-sitter-topos/test/corpus/`.
  - **Issues:** 
    - `Requirements Simple` and `Tasks and Holes` corpus tests are failing.
    - The failure is due to a conflict between the `prose` token (which matches `[^
]+`) and structured lines starting with keywords (e.g., `when:`, `file:`). The scanner attempts to backtrack but the parser precedence logic is not resolving the choice correctly, leading to `(ERROR)` nodes or `(prose)` being matched where a keyword-led rule was expected.
    - `tree-sitter parse debug.tps` shows `(ERROR)` nodes around these constructs.

## Next Steps
1. **Fix Grammar/Scanner Interaction:**
   - The scanner needs to be more robust in identifying when *not* to consume a line as prose. Currently, it peeks for keywords, but maybe the parser needs to guide it more (though Tree-sitter scanners are context-agnostic unless using valid_symbols).
   - Consider simplifying `prose` to just be "rest of line" and handle keywords purely in `grammar.js` if possible, OR refine the `is_keyword` logic in `scanner.c` to be exhaustive and accurate.
   - Investigate why `repeat(choice(...))` in `requirement` rule exits early or mismatches.

2. **Complete Phase 2 (Analysis):**
   - Once grammar parses correctly, implement `topos-analysis` crate.
   - Use `tree-sitter` Rust bindings to traverse the AST.
   - Extract diagnostics (e.g., missing fields, invalid IDs).

3. **Complete Phase 3 (CLI):**
   - Implement `topos check` command.

## Repository State
- **Repo:** `topos` (local)
- **Directory:** `/Users/rand/src/topos`
- **Key Files:**
  - `tree-sitter-topos/grammar.js`
  - `tree-sitter-topos/src/scanner.c`
  - `conductor/tracks/core_grammar_20251220/plan.md`

## Testing
- Run `cd tree-sitter-topos && tree-sitter generate && tree-sitter test` to reproduce failures.
- Run `cd tree-sitter-topos && tree-sitter parse debug.tps` to see parse tree for a sample file.
